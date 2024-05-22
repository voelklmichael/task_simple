/// Each enqueue job yields a ticket which can be used to check if a job is done.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Ticket(usize);
#[derive(Default, Debug)]
pub(super) struct TicketGenerator(usize);
impl TicketGenerator {
    pub(super) fn next(&mut self) -> (Ticket, Ticket) {
        let i = self.0;
        self.0 += 1;
        (Ticket(i), Ticket(i))
    }
}
