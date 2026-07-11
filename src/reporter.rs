use kf_internals::chess::Move;
use kf_internals::report::ResultReport;
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct DatagenReporter {
    sender: Sender<(i32, Move)>,
}

unsafe impl Send for DatagenReporter {}

impl ResultReport for DatagenReporter {
    const REPORT: bool = true;

    fn report_result(&self, eval: i32, mv: Move) {
        let _ = self.sender.send((eval, mv));
    }
}

impl DatagenReporter {
    pub fn new(sender: Sender<(i32, Move)>) -> Self {
        Self { sender }
    }
}
