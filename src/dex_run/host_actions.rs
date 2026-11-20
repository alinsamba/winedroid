use super::*;

impl DexProgram {
    pub(super) fn enqueue_host_action(&self, action: DexHostAction) {
        self.runtime.borrow_mut().host_actions.push(action);
    }

    pub(super) fn take_host_actions(&self) -> Vec<DexHostAction> {
        std::mem::take(&mut self.runtime.borrow_mut().host_actions)
    }
}
