-- Rename credential type discriminant from 'Snmp' to 'SnmpV2c' and remove version field
UPDATE credentials SET credential_type =
    (credential_type - 'version') || jsonb_build_object('type', 'SnmpV2c')
WHERE credential_type->>'type' = 'Snmp';
