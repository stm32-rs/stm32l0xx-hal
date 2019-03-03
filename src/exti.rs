//! External interrupt controller
use crate::bb;
use crate::stm32::EXTI;

pub enum TriggerEdge {
    Rising,
    Falling,
    All,
}

pub trait ExtiExt {
    fn listen(&self, line: u8, edge: TriggerEdge);
    fn unlisten(&self, line: u8);
    fn pend_interrupt(&self, line: u8);
    fn clear_irq(&self, line: u8);
}

impl ExtiExt for EXTI {
    fn listen(&self, line: u8, edge: TriggerEdge) {
        assert!(line < 24);
        match edge {
            TriggerEdge::Rising => bb::set(&self.rtsr, line),
            TriggerEdge::Falling => bb::set(&self.ftsr, line),
            TriggerEdge::All => {
                bb::set(&self.rtsr, line);
                bb::set(&self.ftsr, line);
            }
        }
        bb::set(&self.imr, line);
    }

    fn unlisten(&self, line: u8) {
        assert!(line < 24);
        bb::clear(&self.rtsr, line);
        bb::clear(&self.ftsr, line);
        bb::clear(&self.imr, line);
    }

    fn pend_interrupt(&self, line: u8) {
        assert!(line < 24);
        bb::set(&self.swier, line);
    }

    fn clear_irq(&self, line: u8) {
        assert!(line < 24);
        bb::set(&self.pr, line);
    }
}
