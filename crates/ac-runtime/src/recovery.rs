fn is_meaningful(kind: ProgressKind) -> bool {
    !matches!(kind, ProgressKind::ToolActivity)
}

fn resource_allows(resources: &ResourceSnapshot, active_impl_workers: usize) -> bool {
    match resources.memory_pressure {
        MemoryPressure::Critical => false,
        MemoryPressure::Pressure => active_impl_workers < 1 && resources.active_builds == 0,
        MemoryPressure::Normal => active_impl_workers < 2 && !resources.cpu_busy,
    }
}

fn conflict_blocks(task_id: &StableId, conflicts: &[ConflictPrediction]) -> bool {
    conflicts.iter().any(|conflict| {
        conflict.risk >= 80 && (&conflict.left_task == task_id || &conflict.right_task == task_id)
    })
}

impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}
