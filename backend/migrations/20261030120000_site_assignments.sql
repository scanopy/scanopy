-- Site assignments: one table holding what a user, API key or invite is given on a site, or on all
-- sites. The organization role stays where it is, in `users.permissions`,
-- `user_api_keys.permissions` and `invites.permissions`.
--
-- Where each dropped value goes:
--
--   user_site_access, Member or Viewer user   one row per site, at the user's role
--   user_site_access, Owner or Admin user     one All sites row at Admin (below)
--   user_api_key_site_access                  one row per site, at the key's role, Owner as Admin
--   invites.site_ids, Member or Viewer        one row per site, at the invite's role
--   invites.site_ids, Owner or Admin          one All sites row at Admin (below)
--
-- Every Owner or Admin user and invite gets one All sites row at Admin, which is what the server
-- gave them from the role alone until now. Nobody's access changes.
--
-- Not carried forward:
--   - The site list of an Owner or Admin user or invite, as the plan's backfill table sets out. It
--     grants nothing today, because the role already reaches every site, and the All sites row
--     replaces it.
--   - An `invites.site_ids` entry naming a deleted site. The column has no foreign key, so such an
--     entry can exist, and `site_assignments.site_id` has one, so it can't be copied. It never
--     granted anything: accepting the invite failed on it.
--
-- A key with no sites keeps its role and gets no rows.
--
-- Rows are written once, in one transaction with the drops, so a failure leaves the old tables in
-- place. This release takes downtime: the old image reads the dropped tables, so rolling back means
-- restoring the database. Each `squawk-ignore` below relies on that window.
--
-- A role value other than Owner, Admin or Member reads as Viewer, as `UserOrgPermissions::from_str`
-- reads it.

SET lock_timeout = '5s';
SET statement_timeout = '30s';

CREATE TABLE site_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    api_key_id UUID REFERENCES user_api_keys(id) ON DELETE CASCADE,
    invite_id UUID REFERENCES invites(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('Admin', 'Member', 'Viewer')),
    -- NULL means All sites.
    site_id UUID REFERENCES sites(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT site_assignments_one_principal CHECK (num_nonnulls(user_id, api_key_id, invite_id) = 1)
);

-- One assignment per principal and scope. NULL site ids compare equal here, so a principal holds
-- one All sites row at most.
-- squawk-ignore require-concurrent-index-creation
CREATE UNIQUE INDEX site_assignments_user_scope
    ON site_assignments (user_id, COALESCE(site_id, '00000000-0000-0000-0000-000000000000'))
    WHERE user_id IS NOT NULL;
-- squawk-ignore require-concurrent-index-creation
CREATE UNIQUE INDEX site_assignments_api_key_scope
    ON site_assignments (api_key_id, COALESCE(site_id, '00000000-0000-0000-0000-000000000000'))
    WHERE api_key_id IS NOT NULL;
-- squawk-ignore require-concurrent-index-creation
CREATE UNIQUE INDEX site_assignments_invite_scope
    ON site_assignments (invite_id, COALESCE(site_id, '00000000-0000-0000-0000-000000000000'))
    WHERE invite_id IS NOT NULL;
-- squawk-ignore require-concurrent-index-creation
CREATE INDEX site_assignments_site ON site_assignments (site_id);
-- squawk-ignore require-concurrent-index-creation
CREATE INDEX site_assignments_organization ON site_assignments (organization_id);

-- Owner or Admin users: All sites.
INSERT INTO site_assignments (organization_id, user_id, role, site_id)
SELECT organization_id, id, 'Admin', NULL
FROM users
WHERE permissions IN ('Owner', 'Admin');

-- Member or Viewer users: the sites they were given.
INSERT INTO site_assignments (organization_id, user_id, role, site_id, created_at)
SELECT users.organization_id,
       users.id,
       CASE users.permissions WHEN 'Member' THEN 'Member' ELSE 'Viewer' END,
       access.site_id,
       access.created_at
FROM user_site_access access
JOIN users ON users.id = access.user_id
WHERE users.permissions NOT IN ('Owner', 'Admin');

-- Keys: the sites they were given, whatever the role.
INSERT INTO site_assignments (organization_id, api_key_id, role, site_id, created_at)
SELECT keys.organization_id,
       keys.id,
       CASE
           WHEN keys.permissions IN ('Owner', 'Admin') THEN 'Admin'
           WHEN keys.permissions = 'Member' THEN 'Member'
           ELSE 'Viewer'
       END,
       access.site_id,
       access.created_at
FROM user_api_key_site_access access
JOIN user_api_keys keys ON keys.id = access.api_key_id;

-- Owner or Admin invites: All sites.
INSERT INTO site_assignments (organization_id, invite_id, role, site_id)
SELECT organization_id, id, 'Admin', NULL
FROM invites
WHERE permissions IN ('Owner', 'Admin');

-- Member or Viewer invites: the sites listed, once each, while the site exists.
INSERT INTO site_assignments (organization_id, invite_id, role, site_id)
SELECT DISTINCT invites.organization_id,
       invites.id,
       CASE invites.permissions WHEN 'Member' THEN 'Member' ELSE 'Viewer' END,
       sites.id
FROM invites
CROSS JOIN LATERAL unnest(invites.site_ids) AS listed(site_id)
JOIN sites ON sites.id = listed.site_id
WHERE invites.permissions NOT IN ('Owner', 'Admin');

-- squawk-ignore ban-drop-table
DROP TABLE user_site_access;
-- squawk-ignore ban-drop-table
DROP TABLE user_api_key_site_access;
-- squawk-ignore ban-drop-column
ALTER TABLE invites DROP COLUMN site_ids;
