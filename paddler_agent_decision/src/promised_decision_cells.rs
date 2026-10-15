use std::cell::Cell;
use std::rc::Rc;

pub struct PromisedDecisionCells {
    pub available_cells: Rc<Cell<usize>>,
    pub cells: usize,
}

impl Drop for PromisedDecisionCells {
    fn drop(&mut self) {
        self.available_cells
            .set(self.available_cells.get() + self.cells);
    }
}
