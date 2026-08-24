# Third-Party Notices

Phase 26 establishes the release notice format and requires every shipped dependency,
asset and managed external binary to have a known license/provenance record before
public V1 distribution.

Current release-preparation inventory is generated from workspace dependency audit
records and persisted in schema v17. No copied donor repository code is recorded as
shipped runtime material in this batch.

Required fields per shipped component:

- Name
- Version
- License
- Source
- Checksum or package integrity record
- Notice requirement
- Distribution decision

Unknown or incompatible license status blocks public release.
