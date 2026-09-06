# P01-WP17 Adoption Decisions and Implementation Packets

## Future Interfaces Required

```text
ImplementationPacketRegistry.list(phase, wp) -> PacketIndex
ImplementationPacketRegistry.get(packetId) -> Packet
ArchitectureDecisionLookup.resolve(subsystem) -> AdoptionDecisionSet
```

## Ownership

Docs/progress own extraction status. Implementation phases own code. Kernel architecture remains governed by core docs and ADRs.

## State/Data Ownership

Packets are durable planning evidence. They do not own runtime state, schema, or source code behavior.

## Integration Points

Phase 2+ work packages must cite the relevant packet before implementation and record deviations through ADR/amendment process.

## Security Constraints

Do not copy donor code without license review. Do not use donor paths as runtime dependencies. Preserve untrusted-tool and evidence boundaries.

## Unresolved Questions

Whether to add a machine-readable packet index before Phase 2 begins.
