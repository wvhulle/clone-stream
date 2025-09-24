use std::task::Waker;

use log::{trace, warn};

use crate::{
    error::{CloneStreamError, Result},
    states::CloneState,
};

#[derive(Debug)]
pub(crate) struct CloneRegistry {
    clones: Vec<Option<CloneState>>,
    available_indices: Vec<usize>,
    max_clone_count: usize,
}

impl CloneRegistry {
    pub(crate) fn new(max_clone_count: usize) -> Self {
        Self {
            clones: Vec::new(),
            available_indices: Vec::new(),
            max_clone_count,
        }
    }

    pub(crate) fn register(&mut self) -> Result<usize> {
        if let Some(reused_id) = self.available_indices.pop() {
            trace!("Registering clone {reused_id} (reused index).");
            self.clones[reused_id] = Some(CloneState::default());
            return Ok(reused_id);
        }

        if self.active_count() >= self.max_clone_count {
            return Err(CloneStreamError::MaxClonesExceeded {
                current_count: self.active_count(),
                max_allowed: self.max_clone_count,
            });
        }

        let clone_id = self.clones.len();
        trace!("Registering clone {clone_id} (new index).");
        self.clones.push(Some(CloneState::default()));
        Ok(clone_id)
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        trace!("Unregistering clone {clone_id}.");

        if !self.exists(clone_id) {
            warn!("Attempted to unregister clone {clone_id} that was not registered");
            return;
        }

        self.clones[clone_id] = None;
        self.available_indices.push(clone_id);
        trace!("Unregister of clone {clone_id} complete.");
    }

    pub(crate) fn take(&mut self, clone_id: usize) -> Option<CloneState> {
        self.clones.get_mut(clone_id)?.take()
    }

    pub(crate) fn restore(&mut self, clone_id: usize, state: CloneState) {
        if let Some(slot) = self.clones.get_mut(clone_id) {
            *slot = Some(state);
        }
    }

    pub(crate) fn exists(&self, clone_id: usize) -> bool {
        clone_id < self.clones.len() && self.clones[clone_id].is_some()
    }

    pub(crate) fn active_count(&self) -> usize {
        self.clones.iter().filter(|s| s.is_some()).count()
    }

    pub(crate) fn len(&self) -> usize {
        self.clones.len()
    }

    pub(crate) fn iter_active_with_ids(&self) -> impl Iterator<Item = (usize, &CloneState)> {
        self.clones
            .iter()
            .enumerate()
            .filter_map(|(id, state_opt)| state_opt.as_ref().map(|state| (id, state)))
    }

    pub(crate) fn iter_active(&self) -> impl Iterator<Item = &CloneState> {
        self.clones
            .iter()
            .filter_map(|state_opt| state_opt.as_ref())
    }

    pub(crate) fn collect_wakers_needing_base_item(&self) -> Vec<Waker> {
        trace!("Collecting wakers for clones needing base item.");
        self.iter_active()
            .filter(|state| state.should_still_see_base_item())
            .filter_map(CloneState::waker)
            .collect()
    }

    pub(crate) fn has_other_clones_waiting(&self, exclude_clone_id: usize) -> bool {
        self.clones.iter().enumerate().any(|(clone_id, state_opt)| {
            clone_id != exclude_clone_id
                && state_opt
                    .as_ref()
                    .is_some_and(CloneState::should_still_see_base_item)
        })
    }

    pub(crate) fn get_clone_state(&self, clone_id: usize) -> Option<&CloneState> {
        self.clones.get(clone_id).and_then(|opt| opt.as_ref())
    }

}
