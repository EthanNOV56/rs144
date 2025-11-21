use crate::{Milliseconds, NakedFileDescriptor, system_call};

use libc::{POLLIN, POLLOUT, RawFd, nfds_t, poll, pollfd, pollfd};
use thiserror::Error;

use std::collections::VecDeque;

enum Direction {
    In,
    Out,
}

enum EventResult {
    Success,
    Timeout,
    Exit,
}

#[derive(Error, Debug)]
enum EventLoopError {
    #[error("IO error on file descriptor")]
    IoError,
    #[error("Busy wait detected")]
    BusyWait,
    #[error("Poll system call failed")]
    PollFailed,
}

enum EventAction {
    Continue,
    Remove,
    Exit,
}

pub trait EventHandler: Send {
    fn on_event(&mut self, fd: RawFd, direction: Direction) -> EventAction;
    fn serv_cnt(&self) -> usize;
}

struct EventRule {
    fd: NakedFileDescriptor,
    direction: Direction,
    handler: Box<dyn EventHandler>,
    serv_cnt: usize,
}

impl EventRule {
    pub fn new(
        fd: NakedFileDescriptor,
        direction: Direction,
        handler: Box<dyn EventHandler>,
    ) -> Self {
        EventRule {
            fd,
            direction,
            handler,
            serv_cnt: 0,
        }
    }

    pub fn interest(&self) -> bool {
        !self.fd.eof() && !self.fd.closed()
    }

    pub fn callback(&mut self) {
        match self.handler.on_event(self.fd.into(), self.direction) {
            EventAction::Continue => {}
            EventAction::Remove => {
                todo!();
            }
            EventAction::Exit => {
                todo!();
            }
        }
    }

    pub fn serv_cnt(&self) -> usize {
        match self.direction {
            Direction::In => self.fd.read_count(),
            Direction::Out => self.fd.write_count(),
        }
    }
}

pub struct EventLoop {
    rules: VecDeque<EventRule>,
    to_remove: Vec<usize>,
    should_exit: bool,
}

impl EventLoop {
    pub fn new() -> Self {
        Self {
            rules: VecDeque::new(),
            to_remove: Vec::new(),
            should_exit: false,
        }
    }

    pub fn add_rule(&mut self, rule: EventRule) {
        self.rules.push_back(rule);
    }

    pub fn run(&mut self, time_out: Milliseconds) -> Result<(), EventLoopError> {
        while !self.should_exit {
            match self.wait_next_event(time_out)? {
                EventResult::Exit => break,
                EventResult::Timeout => continue,
                EventResult::Success => {}
            }
        }
        Ok(())
    }

    pub fn wait_next_event(
        &mut self,
        timeout_ms: Milliseconds,
    ) -> Result<EventResult, EventLoopError> {
        self.cleanup_rules();

        if self.rules.is_empty() {
            return Ok(EventResult::Exit);
        }

        let (pollfds, something_to_poll) = self.build_pollfds();

        if !something_to_poll {
            return Ok(EventResult::Exit);
        }

        let ready_count = system_call("poll", || unsafe {
            poll(
                pollfds.as_ptr() as *mut pollfd,
                pollfds.len() as nfds_t,
                timeout_ms,
            )
        })?;

        if ready_count == -1 {
            let errno = std::io::Error::last_os_error();
            if errno.raw_os_error() == Some(libc::EINTR) {
                return Ok(EventResult::Exit);
            }
            return Err(EventLoopError::PollFailed);
        }

        if ready_count == 0 {
            return Ok(EventResult::Timeout);
        }

        self.process_poll_results(&pollfds)?;
        self.cleanup_rules();

        Ok(EventResult::Success)
    }

    fn build_pollfds(&self) -> (Vec<pollfd>, bool) {
        let mut pollfds = Vec::with_capacity(self.rules.len());
        let mut something_to_poll = false;

        for rule in &self.rules {
            let events = if rule.interest() {
                something_to_poll = true;
                match rule.direction {
                    Direction::In => POLLIN as i16,
                    Direction::Out => POLLOUT as i16,
                }
            } else {
                0
            };

            pollfds.push(pollfd {
                fd: rule.fd.fd(),
                events,
                revents: 0,
            });
        }

        (pollfds, something_to_poll)
    }

    fn process_poll_results(&mut self, pollfds: &[pollfd]) -> Result<(), EventLoopError> {
        for (idx, pollfd) in pollfds.iter().enumerate() {
            if pollfd.revents & (POLLERR as i16 | POLLNVAL as i16) != 0 {
                return Err(EventLoopError::IoError);
            }

            let rule = &mut self.rules[idx];
            let poll_ready = pollfd.revents & pollfd.events != 0;
            let poll_hup = pollfd.revents & (POLLHUP as i16) != 0;

            if poll_hup && pollfd.events != 0 && !poll_ready {
                self.to_remove.push(idx);
                continue;
            }

            if poll_ready {
                let count_before = rule.serv_cnt();
                rule.callback();
                if count_before == rule.serv_cnt() && rule.interest() {
                    return Err(EventLoopError::BusyWait);
                }
            }
        }
        Ok(())
    }

    fn cleanup_rules(&mut self) {
        if self.to_remove.is_empty() {
            return;
        }
        self.to_remove.sort_unstable_by(|a, b| b.cmp(a));
        for &idx in &self.to_remove {
            if idx < self.rules.len() {
                self.rules.remove(idx);
            }
        }
        self.to_remove.clear();
    }
}
