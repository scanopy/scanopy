//! The walk engine: `walk_subtree`, `walk_column`, and the retry, desync and degrade helpers.

use super::*;

/// Walk the OID subtree rooted at `base_oid_str`, invoking `on_entry(suffix, value)`
/// for every varbind under it, where `suffix` is the OID sub-ids after the base.
///
/// Uses SNMP `getbulk` for throughput (one round returns up to `BULK_MAX_REPETITIONS`
/// varbinds instead of one per round-trip) and transparently falls back to `getnext`
/// if the agent rejects getbulk (e.g. SNMPv1).
///
/// How many times one walk step will re-issue its request after reading someone else's answer.
///
/// Small deliberately. A retry costs one round trip and only happens on a genuine desync, but a
/// device that is *persistently* answering out of step should be reported as truncated rather
/// than have the scan spin on it.
pub(super) const MAX_DESYNC_RETRIES: u8 = 2;

/// How many times one walk step will re-issue its request after getting no usable answer at all —
/// a timeout, a session error, or a page with no varbinds on it.
///
/// SNMP runs over UDP and nothing beneath us retransmits: `AsyncSession::send_and_recv` is one
/// `send` and one `recv`, bounded only by [`SNMP_TIMEOUT`]. `snmpwalk` defaults to `-r 5 -t 1`, so
/// until this existed the daemon was strictly less tolerant than the command line operators use to
/// prove a device is readable — a single dropped datagram in any one of the seven LLDP columns
/// ended that column, which marks the whole neighbour set non-authoritative and leaves the switch
/// looking as though it has no LLDP at all (GH #685).
///
/// Counted separately from [`MAX_DESYNC_RETRIES`] so a device suffering both faults cannot spend
/// one budget on the other. Kept as small as the desync budget for the same reason: a device that
/// has genuinely stopped answering should be reported, not spun on.
pub(super) const MAX_TRANSPORT_RETRIES: u8 = 2;

/// Ask the agent for half as much next time, and give up on getbulk entirely once even a
/// single-repetition page has not worked.
///
/// Halving rather than dropping straight to getnext because the difference is a whole table's
/// worth of round trips: a 10000-entry FDB read one varbind at a time is the shape the walk
/// timeout was raised for. This is what net-snmp does with `tooBig`, and it is why the reporter's
/// `snmpbulkwalk` read a table our walk gave up on.
fn shrink_page(max_reps: &mut u32, use_bulk: &mut bool) {
    if *max_reps > 1 {
        *max_reps = (*max_reps / 2).max(1);
    } else {
        *use_bulk = false;
    }
}

/// Ask an easier question after the agent answered nothing at all, and make sure the easiest one
/// is reachable.
///
/// [`shrink_page`] alone could not get there. Halving from [`BULK_MAX_REPETITIONS`] needs five
/// steps to put `max_reps` at 1 and a sixth to clear `use_bulk`, while [`MAX_TRANSPORT_RETRIES`]
/// allows two — so on a timeout path getnext was three shrinks out of reach and the column was
/// abandoned having never asked the one question the device would have answered. GH #668's
/// reporter proved it would: `snmpwalk`, which is getnext, read the column their scan reported as
/// empty.
///
/// So the last retry spends itself on getnext rather than on a third bulk page. The budget is
/// unchanged — three requests, at most `3 * SNMP_TIMEOUT` on a column, exactly what it cost
/// before — and the earlier retry still halves, because one lost datagram is the likelier cause
/// the first time and halving is free. Two consecutive silences is where "this agent cannot serve
/// getbulk" starts to beat "a packet went missing".
///
/// Deliberately not applied to `tooBig`: there the agent named page size as the problem and asked
/// for a smaller one, so halving is its own answer and terminates on its own.
fn degrade_after_silence(transport_retries: u8, max_reps: &mut u32, use_bulk: &mut bool) {
    if transport_retries >= MAX_TRANSPORT_RETRIES {
        *use_bulk = false;
    } else {
        shrink_page(max_reps, use_bulk);
    }
}

/// Whether this error means the session read an answer to a question nobody is waiting for.
///
/// The daemon abandons SNMP requests constantly — 5s per query, 60s per walk — and keeps using
/// the session afterwards. `drain_stale` clears the socket before each send, but a response still
/// in flight from an abandoned request lands *after* that drain and is read by the next `recv`,
/// where it fails validation. That is a transient belonging to the previous request, not a
/// verdict on this one, and ending the walk on it turned one slow answer into a truncated table
/// — visible in a customer log as `GET timeout` followed immediately by `RequestIdMismatch`.
///
/// Re-issuing is safe precisely because the failed read *consumed* the stale datagram, so the
/// retry cannot be served the same one again.
fn is_desync(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<snmp2::Error>(),
            Some(snmp2::Error::RequestIdMismatch | snmp2::Error::CommunityMismatch)
        )
    })
}

/// A getnext or get the agent answered with a non-zero `error-status` the caller has no remedy
/// for. Its varbinds only echo the request, so the status is all the response says.
#[derive(Debug, thiserror::Error)]
#[error("agent answered error-status {0}")]
pub struct AgentErrorStatus(pub u32);

fn is_error_status(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.downcast_ref::<AgentErrorStatus>().is_some())
}

/// Returns why the walk stopped. [`WalkStop::is_complete`] is true when the subtree was walked
/// to its natural end (or the agent said it has no such OID) and false when it was cut short by
/// `MAX_WALK_ENTRIES`, a session error, a timeout, a non-advancing OID, or an abnormal empty
/// response — callers that prune against a full table (see `walk_if_table`, GH #649) must treat
/// an incomplete walk as partial. The reason itself is returned rather than collapsed to a bool
/// because "the agent has no such OID" and "the table is implemented and empty" are different
/// answers with different consequences; see [`WalkStop::is_unsupported`].
pub(super) async fn walk_subtree<T, F>(
    session: &mut T,
    ip: IpAddr,
    base_oid_str: &str,
    mut on_entry: F,
) -> Result<WalkStop>
where
    T: SnmpWalkTransport,
    F: FnMut(&[u64], &Value),
{
    let base_parts: Vec<u64> = oids::oid_parts(base_oid_str);

    let mut current_parts = base_parts.clone();
    let mut count = 0usize;
    // Seeded from the session rather than from `true`: an earlier column on this host may already
    // have established that the device does not answer getbulk, and re-establishing it costs
    // `MAX_TRANSPORT_RETRIES + 1` timeouts per column (GH #668).
    let mut use_bulk = !session.getbulk_unusable();
    // Set where this walk turns getbulk off, and acted on at the top of the loop. Deferred rather
    // than called in place because the arms below hold a borrow of `session` for the page they
    // just asked for.
    let mut note_fallback = false;
    let mut stop = WalkStop::EndOfSubtree;
    let mut stop_detail: Option<String> = None;
    let mut desync_retries = 0u8;
    let mut transport_retries = 0u8;
    // Repetitions asked for per getbulk round. Walk-local rather than the constant because it only
    // ever shrinks: an agent that could not fit 20 varbinds will not fit them on the next page
    // either, so re-escalating would just re-earn the same failure.
    let mut max_reps = BULK_MAX_REPETITIONS;
    // Every in-subtree OID already handed to `on_entry`. This is what tells the two devices that
    // used to look identical apart: an OID below where we asked from is the GH #674 firmware bug
    // when it names a row we have not seen, and an agent going in circles when it does not.
    // Bounded by `MAX_WALK_ENTRIES` because nothing is inserted without also counting.
    let mut seen: HashSet<Vec<u64>> = HashSet::new();
    // The highest in-subtree OID accepted so far. The staleness test below has to compare against
    // this rather than the cursor: the cursor now follows the agent's own order and may sit below
    // rows already collected, and comparing against it would let a genuinely stale response pass.
    let mut high_water = base_parts.clone();

    'walk: loop {
        if note_fallback {
            note_fallback = false;
            session.note_getbulk_unusable();
        }
        if count >= MAX_WALK_ENTRIES {
            stop = WalkStop::EntryCap;
            break;
        }

        let varbinds = if use_bulk {
            match session.walk_getbulk(&current_parts, max_reps).await {
                Ok(WalkPage::Varbinds(v)) => v,
                Ok(WalkPage::BulkUnsupported) => {
                    // Agent rejected getbulk (e.g. v1) — retry from the same OID with getnext and
                    // stay on getnext for the rest of this walk, and of every other walk on this
                    // session: a device without getbulk does not acquire it between columns.
                    use_bulk = false;
                    note_fallback = true;
                    continue 'walk;
                }
                Ok(WalkPage::Refused { error_status }) => {
                    // Not a retry against a budget: halving terminates on its own (20 → 10 → 5 →
                    // 2 → 1 → getnext) and each round asks a strictly easier question than the
                    // one just refused. A device that refuses even a single-repetition page does
                    // not serve getbulk, so the rest of the session skips it.
                    shrink_page(&mut max_reps, &mut use_bulk);
                    note_fallback = !use_bulk;
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        error_status,
                        max_repetitions = max_reps,
                        getbulk = use_bulk,
                        "Agent refused the page; asking for less"
                    );
                    continue 'walk;
                }
                Err(e) if is_desync(&e) && desync_retries < MAX_DESYNC_RETRIES => {
                    desync_retries += 1;
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        attempt = desync_retries,
                        error = %e,
                        "Re-issuing after reading a stale answer"
                    );
                    continue 'walk;
                }
                Err(e) if transport_retries < MAX_TRANSPORT_RETRIES => {
                    transport_retries += 1;
                    // Ask something easier, and on the last retry ask the easiest thing there is.
                    // A timeout here is as likely to be the agent labouring over a large page as
                    // a lost datagram — the reporter's switch answered getbulk at roughly nine
                    // times the per-varbind cost of getnext — and both are addressed by asking
                    // for less. See `degrade_after_silence` for why the last one goes all the way.
                    degrade_after_silence(transport_retries, &mut max_reps, &mut use_bulk);
                    note_fallback = !use_bulk;
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        attempt = transport_retries,
                        max_repetitions = max_reps,
                        getbulk = use_bulk,
                        error = %e,
                        "Re-issuing after no answer"
                    );
                    continue 'walk;
                }
                Err(e) => {
                    stop = WalkStop::Transport;
                    stop_detail = Some(e.to_string());
                    break;
                }
            }
        } else {
            match session.walk_getnext(&current_parts).await {
                Ok(v) => v,
                Err(e) if is_desync(&e) && desync_retries < MAX_DESYNC_RETRIES => {
                    desync_retries += 1;
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        attempt = desync_retries,
                        error = %e,
                        "Re-issuing after reading a stale answer"
                    );
                    continue 'walk;
                }
                Err(e) if transport_retries < MAX_TRANSPORT_RETRIES => {
                    transport_retries += 1;
                    debug!(
                        ip = %ip,
                        base = base_oid_str,
                        attempt = transport_retries,
                        error = %e,
                        "Re-issuing after no answer"
                    );
                    continue 'walk;
                }
                Err(e) => {
                    stop = if is_error_status(&e) {
                        WalkStop::ErrorStatus
                    } else {
                        WalkStop::Transport
                    };
                    stop_detail = Some(e.to_string());
                    break;
                }
            }
        };

        // Empty response mid-walk is abnormal (getbulk) or an exhausted column (getnext). Worth
        // one more ask before giving up on the column: every other wrong-shaped answer here — a
        // stale OID, a non-advancing OID, a request-id mismatch — is re-asked, and this one has
        // the same causes. An agent that means it answers the same way again and the column ends
        // as it did before.
        if varbinds.is_empty() {
            if transport_retries < MAX_TRANSPORT_RETRIES {
                transport_retries += 1;
                if use_bulk {
                    // An answer with nothing on it is the same non-answer as a timeout, so it
                    // degrades the same way and reaches getnext on the same retry.
                    degrade_after_silence(transport_retries, &mut max_reps, &mut use_bulk);
                    note_fallback = !use_bulk;
                }
                debug!(
                    ip = %ip,
                    base = base_oid_str,
                    attempt = transport_retries,
                    max_repetitions = max_reps,
                    getbulk = use_bulk,
                    "Re-issuing after an answer with no varbinds on it"
                );
                continue 'walk;
            }
            stop = WalkStop::EmptyResponse;
            break;
        }

        // Process the response, remembering the last in-subtree OID to continue from.
        let mut next_parts: Option<Vec<u64>> = None;
        let mut done = false;
        // Set when the agent answered with an OID belonging to some other question. Re-asking is
        // worth a try before giving up on the column.
        let mut retry_page = false;
        // Rows on this page the walk had not already collected. A page that contributes none is
        // the agent repeating itself, which is the one shape that cannot terminate on its own.
        let mut page_new = 0usize;
        for (resp_parts, value) in varbinds {
            if matches!(
                value,
                Value::EndOfMibView | Value::NoSuchObject | Value::NoSuchInstance
            ) {
                stop = WalkStop::EndOfMibView;
                done = true;
                break;
            }
            if resp_parts.len() <= base_parts.len() || !resp_parts.starts_with(&base_parts) {
                // Out of the subtree. That is the natural end of a column *if* the agent moved
                // forward past it — a walk always advances. An OID that doesn't exceed where we
                // asked from is not a continuation of this walk at all (a stale response left
                // over from a cancelled request reads exactly like this), and calling it a
                // natural end would report a column that stopped early as authoritative, which
                // then re-enables the server-side prune #649 exists to suppress.
                if resp_parts <= high_water {
                    stop_detail = Some(format!("responded with {resp_parts:?}"));
                    stop = WalkStop::StaleResponse;
                    // Retryable for the same reason a request-id mismatch is: the answer belongs
                    // to an earlier question and has now been consumed, so re-asking gets a fresh
                    // one. Unlike that case there is no transport error — the response was valid
                    // and carried the wrong OID, which is what a forking `pass` handler under
                    // load produces.
                    retry_page = true;
                } else {
                    stop = WalkStop::EndOfSubtree;
                }
                done = true;
                break;
            }
            // In the subtree, so this is a row of the table being walked — whether or not it
            // ascends. Firmware that stores a table unsorted serves real rows in a real order
            // that simply is not numeric (GH #674); refusing them read part of the reporter's
            // switch and reported the rest as absent. Identity, not ordering, is what separates
            // that from an agent looping: a row already collected is a repeat and is dropped,
            // which also keeps a re-asked page from emitting its rows twice.
            if seen.insert(resp_parts.clone()) {
                if resp_parts > high_water {
                    high_water.clone_from(&resp_parts);
                }
                on_entry(&resp_parts[base_parts.len()..], &value);
                count += 1;
                page_new += 1;
            }
            // Continue from where the agent left off in its own order, which is what lets the
            // rest of an out-of-order table be reached at all.
            next_parts = Some(resp_parts);
            if count >= MAX_WALK_ENTRIES {
                stop = WalkStop::EntryCap;
                done = true;
                break;
            }
        }
        if !done {
            match next_parts {
                Some(parts) => {
                    // The walk must keep making progress — but progress is new rows, not a
                    // larger OID. A device that answers with a tail OID that doesn't
                    // lexicographically exceed the one we asked from (observed on Ubiquiti
                    // bridge-FDB) would otherwise have us re-request the same page until
                    // MAX_WALK_ENTRIES or the integration timeout; it still does, because its
                    // second identical page contributes nothing new. Testing the tail OID
                    // instead used to catch out-of-order firmware in the same net (#674),
                    // which had us discard rows that were there for the asking.
                    if page_new == 0 {
                        stop_detail = Some(format!("responded with {parts:?}"));
                        stop = WalkStop::NonAdvancingOid;
                        retry_page = true;
                        done = true;
                    } else {
                        // The budgets are documented as what one walk *step* may spend and were
                        // being spent across the whole walk, which matters now that reaching
                        // getnext costs both transport retries: without this the fallback arrives
                        // with nothing left, and the first lost datagram after it ends the column
                        // — the exact fragility the budget was added to remove (GH #685). Every
                        // reset is paid for by new rows, so the walk stays bounded by
                        // MAX_WALK_ENTRIES and its own timeout.
                        desync_retries = 0;
                        transport_retries = 0;
                        current_parts = parts;
                    }
                }
                None => {
                    stop = WalkStop::EmptyResponse;
                    done = true;
                }
            }
        }

        // Both wrong-OID shapes converge here — the one detected inside the page and the one
        // detected on the tail — so a retry covers each.
        if retry_page && desync_retries < MAX_DESYNC_RETRIES {
            desync_retries += 1;
            debug!(
                ip = %ip,
                base = base_oid_str,
                attempt = desync_retries,
                ?stop,
                detail = stop_detail.as_deref().unwrap_or(""),
                "Re-asking after an answer that belonged to another request"
            );
            // Re-asking cannot duplicate rows, because `seen` drops any the callback already
            // took. It used to be able to: the wrong-OID varbind is rejected before the
            // callback, but the in-subtree ones ahead of it on the same page were not, and the
            // cursor had not moved — so a re-ask re-delivered them, pushing duplicate VLANs and
            // duplicate per-port memberships into the collectors that append rather than key.
            stop = WalkStop::EndOfSubtree;
            stop_detail = None;
            continue 'walk;
        }
        if done {
            break;
        }
    }

    // A truncated column is why interfaces and neighbours go missing, and the reason is otherwise
    // invisible — a timeout and a session reading stale answers produce identical data. Kept at
    // debug rather than info: truncation is common enough on a busy agent to be noise at info
    // (the SNMP simulator alone produces several per scan), and the operator-facing signal is
    // already the session warning. This is the follow-up detail for when that warning needs
    // explaining. A clean walk stays silent either way.
    if stop.is_truncation() {
        // `ip` is not decoration. This is the only line that says *why* a column came up
        // short, and without the address it cannot be tied to a device — an operator
        // grepping their daemon log for the switch named in the scan warning filtered out
        // every one of these, which is what made a Ubiquiti bridge-FDB failure take two
        // rounds of logs to narrow. Threading it as a parameter rather than relying on an
        // enclosing span means a new walk cannot be added without supplying it.
        debug!(
            ip = %ip,
            base = base_oid_str,
            ?stop,
            detail = stop_detail.as_deref().unwrap_or(""),
            entries = count,
            "SNMP walk truncated"
        );
    }

    Ok(stop)
}

/// Walk one column, folding its outcome into `shortfall`.
///
/// Every multi-column query needs this and none of them had it: `walk_subtree` never returns
/// `Err`, so the `?` these call sites used was dead code and a truncated column was invisible.
pub(super) async fn walk_column<T, F>(
    session: &mut T,
    ip: IpAddr,
    base_oid_str: &str,
    shortfall: &mut Shortfall,
    on_entry: F,
) -> WalkStop
where
    T: SnmpWalkTransport,
    F: FnMut(&[u64], &Value),
{
    let stop = walk_subtree(session, ip, base_oid_str, on_entry)
        .await
        .unwrap_or(WalkStop::Transport);
    shortfall.record(stop);
    stop
}
