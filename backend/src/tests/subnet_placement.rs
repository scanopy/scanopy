//! Placing an address against a site's real subnet list.
//!
//! Against a database rather than a hand-built list, because the list is the whole point. Every
//! site is seeded with an `Internet` and a `Remote Network` subnet, both `0.0.0.0/0`, by
//! `SiteService::create_organizational_subnets` — and a guard tested against a list somebody
//! assembled by hand never sees them. That is exactly how `place_address` shipped returning
//! `Unplaceable` for every address in no real subnet: `0.0.0.0/0` contains everything, so the
//! "is this already held?" check answered yes for every address on earth and no range was ever
//! inferred.
//!
//! These seed the organizational subnets the way a real site gets them, so the list under test
//! is the list production has.

use uuid::Uuid;

use crate::server::shared::attribution::AttributeSource;
use crate::server::{
    auth::middleware::auth::AuthenticatedEntity,
    shared::{
        services::{factory::ServiceFactory, traits::CrudService},
        storage::traits::Storage,
        types::entities::EntitySource,
    },
    subnets::{
        r#impl::base::{Subnet, SubnetCidr, SubnetCidrValue},
        service::Placement,
    },
};

use super::{organization, site, test_services};

/// Seed a site with the two organizational catch-alls a real one is created with, plus whatever
/// else the test wants it to hold.
async fn site_holding(
    cidrs: &[&str],
) -> (
    ServiceFactory,
    uuid::Uuid,
    testcontainers::ContainerAsync<testcontainers::GenericImage>,
) {
    let (storage, services, container) = test_services().await;

    let org = organization();
    storage.organizations.create(&org).await.unwrap();
    let net = site(&org.id);
    storage.sites.create(&net).await.unwrap();

    services
        .site_service
        .create_organizational_subnets(net.id, AuthenticatedEntity::System)
        .await
        .unwrap();

    for cidr in cidrs {
        let mut subnet = super::subnet(&net.id);
        subnet.base.cidr = SubnetCidr::new(
            SubnetCidrValue(cidr.parse().expect("valid test CIDR")),
            AttributeSource::DaemonSelfReport,
        );
        subnet.base.name = (*cidr).to_string();
        services
            .subnet_service
            .create(subnet, AuthenticatedEntity::System)
            .await
            .unwrap();
    }

    (services, net.id, container)
}

/// Record a range the way discovery does — a daemon's own interface, or a device's own netmask.
/// Both reach the server through `Subnet::from_discovery`, which stamps `Observed`.
async fn observe(services: &ServiceFactory, site_id: Uuid, cidr: &str) {
    let mut subnet = super::subnet(&site_id);
    subnet.base.cidr = SubnetCidr::new(
        SubnetCidrValue(cidr.parse().expect("valid test CIDR")),
        // What `Subnet::from_discovery` stamps: a daemon reading its own interface.
        AttributeSource::DaemonSelfReport,
    );
    subnet.base.name = cidr.to_string();
    subnet.base.source = EntitySource::Discovery;
    services
        .subnet_service
        .create(subnet, AuthenticatedEntity::System)
        .await
        .expect("the reading is recorded");
}

async fn live_subnet(services: &ServiceFactory, id: Uuid) -> Subnet {
    services
        .subnet_service
        .get_by_id(&id)
        .await
        .unwrap()
        .expect("the subnet is live")
}

/// Every range the site holds, catch-alls excluded.
async fn held_ranges(services: &ServiceFactory, site_id: Uuid) -> Vec<String> {
    all_live(services, site_id)
        .await
        .into_iter()
        .filter(|s| !s.is_organizational_subnet())
        .map(|s| s.base.cidr.to_string())
        .collect()
}

async fn inferred_ranges(services: &ServiceFactory, site_id: Uuid) -> Vec<Subnet> {
    all_live(services, site_id)
        .await
        .into_iter()
        .filter(|s| s.base.cidr.source() == AttributeSource::LldpNeighbourAddress)
        .collect()
}

async fn all_live(services: &ServiceFactory, site_id: Uuid) -> Vec<Subnet> {
    services
        .subnet_service
        .get_all(
            crate::server::shared::storage::filter::StorableFilter::<Subnet>::new_from_site_ids(&[
                site_id,
            ])
            .live(),
        )
        .await
        .unwrap()
}

/// An address filed on a subnet, so a correction has something to displace.
async fn ip_on(services: &ServiceFactory, site_id: Uuid, subnet_id: Uuid, address: &str) -> Uuid {
    use crate::server::ip_addresses::r#impl::base::{IPAddress, IPAddressBase};

    // The address needs a host to hang off; the FK is not optional.
    let host = services
        .host_service
        .create(super::host(&site_id), AuthenticatedEntity::System)
        .await
        .expect("the host is stored");

    let ip = IPAddress::new(IPAddressBase {
        site_id,
        host_id: host.id,
        subnet_id,
        ip_address: address.parse().expect("valid test address"),
        mac_address: None,
        name: None,
        position: 0,
    });
    services
        .ip_address_service
        .create(ip.clone(), AuthenticatedEntity::System)
        .await
        .expect("the address is stored");
    ip.id
}

async fn live_ip(
    services: &ServiceFactory,
    id: Uuid,
) -> crate::server::ip_addresses::r#impl::base::IPAddress {
    services
        .ip_address_service
        .get_by_id(&id)
        .await
        .unwrap()
        .expect("the address is live")
}

/// The defect Maya hit: an address in a range nothing holds must produce one, not come back
/// unplaceable because two `0.0.0.0/0` rows technically contain it.
#[tokio::test]
async fn an_address_in_no_held_range_infers_one_despite_the_catch_alls() {
    let (services, site_id, _container) = site_holding(&["192.168.4.0/22"]).await;

    let placement = services
        .subnet_service
        .place_address(site_id, "10.20.30.24".parse().unwrap())
        .await
        .unwrap();

    let Placement::Inferred(subnet_id) = placement else {
        panic!("expected a range to be inferred, got {placement:?}");
    };

    let created = services
        .subnet_service
        .get_by_id(&subnet_id)
        .await
        .unwrap()
        .expect("the inferred subnet");
    assert_eq!(created.base.cidr.to_string(), "10.20.30.0/24");
    assert_eq!(
        created.base.cidr.source(),
        AttributeSource::LldpNeighbourAddress
    );
}

/// A real subnet still wins, and is never displaced by a catch-all that also contains the address.
#[tokio::test]
async fn an_address_a_real_subnet_holds_is_placed_there() {
    let (services, site_id, _container) = site_holding(&["192.168.4.0/22"]).await;

    let placement = services
        .subnet_service
        .place_address(site_id, "192.168.7.252".parse().unwrap())
        .await
        .unwrap();

    let Placement::Existing(subnet_id) = placement else {
        panic!("expected the existing subnet, got {placement:?}");
    };

    let held = services
        .subnet_service
        .get_by_id(&subnet_id)
        .await
        .unwrap()
        .expect("the holding subnet");
    assert_eq!(held.base.cidr.to_string(), "192.168.4.0/22");
}

/// Two addresses in one unknown range converge on a single subnet without being pooled — the
/// property that makes placing one address at a time equivalent to the pooled pass.
#[tokio::test]
async fn two_addresses_in_one_unknown_range_converge_on_one_subnet() {
    let (services, site_id, _container) = site_holding(&[]).await;

    let first = services
        .subnet_service
        .place_address(site_id, "10.20.30.11".parse().unwrap())
        .await
        .unwrap();
    let second = services
        .subnet_service
        .place_address(site_id, "10.20.30.240".parse().unwrap())
        .await
        .unwrap();

    let (Placement::Inferred(a), Placement::Inferred(b) | Placement::Existing(b)) = (first, second)
    else {
        panic!("expected both to land in an inferred range, got {first:?} then {second:?}");
    };
    assert_eq!(
        a, b,
        "the second address must reuse the range the first created"
    );

    let inferred: Vec<Subnet> = services
        .subnet_service
        .get_all(
            crate::server::shared::storage::filter::StorableFilter::<Subnet>::new_from_site_ids(&[
                site_id,
            ])
            .live(),
        )
        .await
        .unwrap()
        .into_iter()
        .filter(|s| s.base.cidr.source() == AttributeSource::LldpNeighbourAddress)
        .collect();
    assert_eq!(inferred.len(), 1, "one range, not one per address");
}

/// A public address is not a segment of this site to invent, whatever the catch-alls say.
#[tokio::test]
async fn a_public_address_is_unplaceable_rather_than_invented() {
    let (services, site_id, _container) = site_holding(&["192.168.4.0/22"]).await;

    let placement = services
        .subnet_service
        .place_address(site_id, "8.8.8.8".parse().unwrap())
        .await
        .unwrap();

    assert!(
        matches!(placement, Placement::Unplaceable),
        "got {placement:?}"
    );
}

/// A reading that agrees with the assumed range settles it. Outcome 2: nothing moves, the badge
/// goes.
///
/// This is the one that never worked. `SubnetService::create` cloned the row it matched and threw
/// the incoming rung away, so a daemon reading the real netmask for a segment changed nothing and
/// `Inferred` could only ever be cleared by hand.
#[tokio::test]
async fn a_reading_of_the_same_range_settles_it() {
    let (services, site_id, _container) = site_holding(&[]).await;

    let Placement::Inferred(subnet_id) = services
        .subnet_service
        .place_address(site_id, "10.20.30.24".parse().unwrap())
        .await
        .unwrap()
    else {
        panic!("expected a range to be inferred");
    };

    observe(&services, site_id, "10.20.30.0/24").await;

    let settled = live_subnet(&services, subnet_id).await;
    assert_eq!(settled.base.cidr.to_string(), "10.20.30.0/24");
    assert_eq!(
        settled.base.cidr.source(),
        AttributeSource::DaemonSelfReport
    );
    assert_eq!(
        inferred_ranges(&services, site_id).await.len(),
        0,
        "nothing is still assumed"
    );
}

/// Outcome 3: the segment is wider than assumed, so the row widens in place rather than a second
/// overlapping subnet appearing beside it.
#[tokio::test]
async fn a_reading_that_covers_the_guess_widens_it_in_place() {
    let (services, site_id, _container) = site_holding(&[]).await;

    let Placement::Inferred(subnet_id) = services
        .subnet_service
        .place_address(site_id, "10.20.30.24".parse().unwrap())
        .await
        .unwrap()
    else {
        panic!("expected a range to be inferred");
    };

    observe(&services, site_id, "10.20.30.0/23").await;

    let widened = live_subnet(&services, subnet_id).await;
    assert_eq!(widened.base.cidr.to_string(), "10.20.30.0/23");
    assert_eq!(
        widened.base.cidr.source(),
        AttributeSource::DaemonSelfReport
    );
    assert_eq!(
        held_ranges(&services, site_id).await,
        vec!["10.20.30.0/23".to_string()],
        "one row, corrected — not two overlapping ones"
    );
}

/// Outcome 5: the widening was wrong. The row narrows to what was read, and the addresses it no
/// longer covers are re-filed rather than left pointing at a range without them.
#[tokio::test]
async fn a_reading_inside_the_guess_narrows_it_and_refiles_the_rest() {
    let (services, site_id, _container) = site_holding(&[]).await;

    // Two addresses a /24 apart, so the range that holds both has to be wider than either.
    let Placement::Inferred(subnet_id) = services
        .subnet_service
        .place_address(site_id, "10.20.30.24".parse().unwrap())
        .await
        .unwrap()
    else {
        panic!("expected a range to be inferred");
    };
    let inside = ip_on(&services, site_id, subnet_id, "10.20.30.24").await;
    let outside = ip_on(&services, site_id, subnet_id, "10.20.31.9").await;

    // A netmask for the lower half only.
    observe(&services, site_id, "10.20.30.0/25").await;

    let narrowed = live_subnet(&services, subnet_id).await;
    assert_eq!(narrowed.base.cidr.to_string(), "10.20.30.0/25");
    assert_eq!(
        narrowed.base.cidr.source(),
        AttributeSource::DaemonSelfReport
    );

    assert_eq!(
        live_ip(&services, inside).await.base.subnet_id,
        subnet_id,
        "an address the corrected range still covers stays put"
    );
    let moved = live_ip(&services, outside).await;
    assert_ne!(
        moved.base.subnet_id, subnet_id,
        "an address it no longer covers is re-filed"
    );
    let new_home = live_subnet(&services, moved.base.subnet_id).await;
    assert!(
        new_home.base.cidr.contains(&"10.20.31.9".parse().unwrap()),
        "and re-filed somewhere that actually holds it, not anywhere"
    );
}

/// Outcome 4: a reading covering two guesses says they are one segment, but resolving that means
/// deleting rows — so discovery declines and leaves them for a person.
#[tokio::test]
async fn a_reading_covering_two_guesses_corrects_neither() {
    let (services, site_id, _container) = site_holding(&[]).await;

    for address in ["10.20.30.24", "10.20.31.9"] {
        let Placement::Inferred(_) = services
            .subnet_service
            .place_address(site_id, address.parse().unwrap())
            .await
            .unwrap()
        else {
            panic!("expected a range to be inferred for {address}");
        };
    }

    observe(&services, site_id, "10.20.30.0/23").await;

    let mut held = held_ranges(&services, site_id).await;
    held.sort();
    assert_eq!(
        held,
        vec![
            "10.20.30.0/23".to_string(),
            "10.20.30.0/24".to_string(),
            "10.20.31.0/24".to_string()
        ],
        "the reading is recorded beside the guesses rather than swallowing one of them"
    );
    assert_eq!(
        inferred_ranges(&services, site_id).await.len(),
        2,
        "both stay assumed, and both stay badged for a person to merge"
    );
}

/// The resolution for outcome 4: a person folds an assumed range into the read one that covers it.
///
/// Total by construction — containment guarantees every address fits — so nothing is re-placed and
/// nothing is orphaned.
#[tokio::test]
async fn merging_a_guess_into_the_range_that_covers_it_moves_its_addresses() {
    let (services, site_id, _container) = site_holding(&[]).await;

    // Two guesses, so the reading below covers both and discovery declines to correct either —
    // which is the only state this merge exists to resolve. With one guess it would have been
    // corrected in place and there would be nothing to merge.
    for address in ["10.20.30.24", "10.20.31.9"] {
        let Placement::Inferred(_) = services
            .subnet_service
            .place_address(site_id, address.parse().unwrap())
            .await
            .unwrap()
        else {
            panic!("expected a range to be inferred for {address}");
        };
    }
    let guess = all_live(&services, site_id)
        .await
        .into_iter()
        .find(|s| s.base.cidr.to_string() == "10.20.31.0/24")
        .expect("the second guess")
        .id;
    let address = ip_on(&services, site_id, guess, "10.20.31.9").await;

    observe(&services, site_id, "10.20.30.0/23").await;
    let covering = all_live(&services, site_id)
        .await
        .into_iter()
        .find(|s| s.base.cidr.to_string() == "10.20.30.0/23")
        .expect("the read range");

    services
        .subnet_service
        .merge_into(guess, covering.id, AuthenticatedEntity::System)
        .await
        .expect("the merge succeeds");

    assert_eq!(
        live_ip(&services, address).await.base.subnet_id,
        covering.id,
        "the address moved to the covering range"
    );
    let mut held = held_ranges(&services, site_id).await;
    held.sort();
    assert_eq!(
        held,
        vec!["10.20.30.0/23".to_string(), "10.20.30.0/24".to_string()],
        "the merged guess is gone; the other is still there to be merged in its turn"
    );
}

/// A range is only ever folded into one that contains it. Anything else would be guessing where the
/// addresses that do not fit should go.
#[tokio::test]
async fn a_range_cannot_be_merged_into_one_that_does_not_contain_it() {
    let (services, site_id, _container) = site_holding(&[]).await;

    let Placement::Inferred(guess) = services
        .subnet_service
        .place_address(site_id, "10.20.30.24".parse().unwrap())
        .await
        .unwrap()
    else {
        panic!("expected a range to be inferred");
    };

    observe(&services, site_id, "192.168.4.0/22").await;
    let elsewhere = all_live(&services, site_id)
        .await
        .into_iter()
        .find(|s| s.base.cidr.to_string() == "192.168.4.0/22")
        .expect("the unrelated range");

    assert!(
        services
            .subnet_service
            .merge_into(guess, elsewhere.id, AuthenticatedEntity::System)
            .await
            .is_err()
    );

    // And never into a catch-all, which contains everything by construction.
    let internet = all_live(&services, site_id)
        .await
        .into_iter()
        .find(|s| s.is_organizational_subnet())
        .expect("a seeded catch-all");
    assert!(
        services
            .subnet_service
            .merge_into(guess, internet.id, AuthenticatedEntity::System)
            .await
            .is_err()
    );
}

/// A narrowing with nowhere to put what it displaces is declined, not forced through.
///
/// `10.20.30.0/24` narrowed to `/28` strands `10.20.30.24`: the range it would be re-filed under is
/// its conventional `/24`, and that now overlaps the corrected `/28`, so inference refuses it. The
/// row would end up naming a subnet that does not contain it — stable, but wrong, and invisible to
/// anything but `needs_placement`.
///
/// So the reading is recorded beside the guess instead. The person keeps the range they asked for,
/// the guess stays assumed and still resolvable, and no address names a subnet without it.
#[tokio::test]
async fn a_narrowing_that_would_strand_an_address_is_declined() {
    let (services, site_id, _container) = site_holding(&[]).await;

    let Placement::Inferred(guess) = services
        .subnet_service
        .place_address(site_id, "10.20.30.11".parse().unwrap())
        .await
        .unwrap()
    else {
        panic!("expected a range to be inferred");
    };
    let low = ip_on(&services, site_id, guess, "10.20.30.11").await;
    let high = ip_on(&services, site_id, guess, "10.20.30.24").await;

    // Covers .11 but not .24, whose only home would be a /24 overlapping this very range.
    observe(&services, site_id, "10.20.30.0/28").await;

    let untouched = live_subnet(&services, guess).await;
    assert_eq!(
        untouched.base.cidr.to_string(),
        "10.20.30.0/24",
        "the guess is left alone rather than narrowed onto its own addresses"
    );
    assert_eq!(
        untouched.base.cidr.source(),
        AttributeSource::LldpNeighbourAddress
    );

    let mut held = held_ranges(&services, site_id).await;
    held.sort();
    assert_eq!(
        held,
        vec!["10.20.30.0/24".to_string(), "10.20.30.0/28".to_string()],
        "the reading is recorded beside it"
    );

    for id in [low, high] {
        let address = live_ip(&services, id).await;
        let named = live_subnet(&services, address.base.subnet_id).await;
        assert!(
            named.base.cidr.contains(&address.base.ip_address),
            "{} names {}, which does not contain it",
            address.base.ip_address,
            named.base.cidr
        );
    }
}
