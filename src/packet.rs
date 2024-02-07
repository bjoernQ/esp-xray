use super::*;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Cause {
    Idle, // ??? ids?
    Sleep,
}

/// Events from the target
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Event<'a> {
    Overflow {
        dropped_packets: u32,
        ts_delta: u32,
    } = 1,
    IsrEnter {
        isr: u8,
        ts_delta: u32,
    },
    IsrExit {
        ts_delta: u32,
    },
    TaskStartExec {
        task: u32,
        ts_delta: u32,
    },
    TaskStopExec {
        ts_delta: u32,
    },
    TaskStartReady {
        task: u32,
        ts_delta: u32,
    },
    TaskStopReady {
        task: u32,
        cause: Cause,
        ts_delta: u32,
    },
    TaskCreate {
        task: u32,
        ts_delta: u32,
    },
    TaskInfo {
        task: u32,
        prio: u32,
        name: &'a str,
        ts_delta: u32,
    },
    TraceStart {
        ts_delta: u32,
    },
    TraceStop {
        ts_delta: u32,
    },
    SystimeCycles {
        time: u32,
        ts_delta: u32,
    },
    SystimeUs {
        time: u64,
        ts_delta: u32,
    },
    UserStart {
        user_id: u32,
        ts_delta: u32,
    } = 15,
    UserStop {
        user_id: u32,
        ts_delta: u32,
    },
    Idle {
        ts_delta: u32,
    },
    IsrToScheduler {
        ts_delta: u32,
    },
    TimerEnter {
        timer_id: u32,
        ts_delta: u32,
    },
    TimerExit {
        ts_delta: u32,
    },
    StackInfo {
        task_id: u32,
        stack_base: u32,
        stack_size: u32,
        ts_delta: u32,
    },
    Init {
        sys_freq: u32,
        cpu_freq: u32,
        ram_base: u32,
        id_shift: u32,
        ts_delta: u32,
    } = 24,
    NameResource {
        resource_id: u32,
        name: &'a str,
        ts_delta: u32,
    },
    PrintFormatted {
        s: &'a str,
        ts_delta: u32,
    },
    NumModules {
        modules: u32,
        ts_delta: u32,
    },
    EndCall {
        event_id: u32,
        ts_delta: u32,
    },
    TaskTerminate {
        task_id: u32,
        ts_delta: u32,
    },
}

impl<'a> Event<'a> {
    fn discriminant(&self) -> u8 {
        unsafe { *(self as *const Self as *const u8) }
    }

    pub fn encode(&self, buffer: &mut [u8]) -> Result<usize, Error> {
        buffer[0] = self.discriminant();
        let mut count = 1;

        // ids > 24 have a length - use a placeholder of one byte which means max 0x7f (excluding event_id, length and timestamp)
        if self.discriminant() >= 24 {
            buffer[1] = 0;
            count += 1;
        }

        let ts_delta = match self {
            Event::Overflow {
                dropped_packets,
                ts_delta,
            } => {
                count = encode_u32(*dropped_packets, buffer, count);
                ts_delta
            }
            Event::IsrEnter { isr, ts_delta } => {
                count = encode_u32(*isr as u32, buffer, count);
                ts_delta
            }
            Event::IsrExit { ts_delta } => ts_delta,
            Event::TaskStartExec { task, ts_delta } => {
                count = encode_u32(*task as u32, buffer, count);
                ts_delta
            }
            Event::TaskStopExec { ts_delta } => ts_delta,
            Event::TaskStartReady { task, ts_delta } => {
                count = encode_u32(*task as u32, buffer, count);
                ts_delta
            }
            Event::TaskStopReady {
                task,
                cause,
                ts_delta,
            } => {
                count = encode_u32(*task as u32, buffer, count);
                count = encode_u32(*cause as u32, buffer, count);
                ts_delta
            }
            Event::TaskCreate { task, ts_delta } => {
                count = encode_u32(*task as u32, buffer, count);
                ts_delta
            }
            Event::TaskInfo {
                task,
                prio,
                name,
                ts_delta,
            } => {
                count = encode_u32(*task as u32, buffer, count);
                count = encode_u32(*prio as u32, buffer, count);
                count = encode_str(*name, buffer, count);
                ts_delta
            }
            Event::TraceStart { ts_delta } => ts_delta,
            Event::TraceStop { ts_delta } => ts_delta,
            Event::SystimeCycles { time, ts_delta } => {
                count = encode_u32(*time as u32, buffer, count);
                ts_delta
            }
            Event::SystimeUs { time, ts_delta } => {
                count = encode_u32(*time as u32, buffer, count);
                count = encode_u32((*time >> 32) as u32, buffer, count);
                ts_delta
            }
            Event::UserStart { user_id, ts_delta } => {
                count = encode_u32(*user_id as u32, buffer, count);
                ts_delta
            }
            Event::UserStop { user_id, ts_delta } => {
                count = encode_u32(*user_id as u32, buffer, count);
                ts_delta
            }
            Event::Idle { ts_delta } => ts_delta,
            Event::IsrToScheduler { ts_delta } => ts_delta,
            Event::TimerEnter { timer_id, ts_delta } => {
                count = encode_u32(*timer_id as u32, buffer, count);
                ts_delta
            }
            Event::TimerExit { ts_delta } => ts_delta,
            Event::StackInfo {
                task_id,
                stack_base,
                stack_size,
                ts_delta,
            } => {
                count = encode_u32(*task_id as u32, buffer, count);
                count = encode_u32(*stack_base as u32, buffer, count);
                count = encode_u32(*stack_size as u32, buffer, count);
                ts_delta
            }
            Event::Init {
                sys_freq,
                cpu_freq,
                ram_base,
                id_shift,
                ts_delta,
            } => {
                count = encode_u32(*sys_freq as u32, buffer, count);
                count = encode_u32(*cpu_freq as u32, buffer, count);
                count = encode_u32(*ram_base as u32, buffer, count);
                count = encode_u32(*id_shift as u32, buffer, count);
                ts_delta
            }
            Event::NameResource {
                resource_id,
                name,
                ts_delta,
            } => {
                count = encode_u32(*resource_id as u32, buffer, count);
                count = encode_str(*name, buffer, count);
                ts_delta
            }
            Event::PrintFormatted { s, ts_delta } => {
                count = encode_str(*s, buffer, count);
                ts_delta
            }
            Event::NumModules { modules, ts_delta } => {
                count = encode_u32(*modules as u32, buffer, count);
                ts_delta
            }
            Event::EndCall { event_id, ts_delta } => {
                count = encode_u32(*event_id as u32, buffer, count);
                ts_delta
            }
            Event::TaskTerminate { task_id, ts_delta } => {
                count = encode_u32(*task_id as u32, buffer, count);
                ts_delta
            }
        };

        if self.discriminant() >= 24 {
            buffer[1] = (count - 2) as u8;
        }

        count = encode_u32(*ts_delta, buffer, count);
        Ok(count)
    }
}

fn encode_u32(mut value: u32, buffer: &mut [u8], mut count: usize) -> usize {
    while value > 0x7F {
        buffer[count] = (value | 0x80) as u8;
        count += 1;
        value >>= 7;
    }
    buffer[count] = value as u8;
    count += 1;

    count
}

fn encode_str(s: &str, buffer: &mut [u8], mut count: usize) -> usize {
    count = encode_u32(s.len() as u32, buffer, count);
    for c in s.chars().into_iter() {
        buffer[count] = c as u8;
        count += 1;
    }

    count
}

/// Commands sent by host
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Command {
    Start = 1,
    Stop,
    GetSysTime,
}

impl TryFrom<u8> for Command {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Command::Start),
            2 => Ok(Command::Stop),
            3 => Ok(Command::GetSysTime),
            _ => Err(Error::UnknownCommand),
        }
    }
}

mod test {
    #[allow(unused)]
    use super::*;

    #[test]
    fn test_encode_500() {
        let mut buffer = [0u8; 10];
        let count = encode_u32(500, &mut buffer, 0);
        assert_eq!(&[0xf4, 0x03], &buffer[..count]);
    }

    #[test]
    fn test_encode_0x7000() {
        let mut buffer = [0u8; 10];
        let count = encode_u32(0x7000, &mut buffer, 0);
        assert_eq!(&[0x80, 0xE0, 0x01], &buffer[..count]);
    }

    #[test]
    fn test_encode_overflow() {
        let mut buffer = [0u8; 10];
        let count = Event::Overflow {
            dropped_packets: 0x10,
            ts_delta: 0x50,
        }
        .encode(&mut buffer)
        .unwrap();
        assert_eq!(&[0x01, 0x10, 0x50], &buffer[..count]);
    }

    #[test]
    fn test_encode_isr_enter() {
        let mut buffer = [0u8; 10];
        let count = Event::IsrEnter {
            isr: 15,
            ts_delta: 80,
        }
        .encode(&mut buffer)
        .unwrap();
        assert_eq!(&[0x02, 0x0f, 0x50], &buffer[..count]);
    }

    #[test]
    fn test_encode_init() {
        let mut buffer = [0u8; 10];
        let count = Event::Init {
            sys_freq: 1,
            cpu_freq: 2,
            ram_base: 3,
            id_shift: 4,
            ts_delta: 80,
        }
        .encode(&mut buffer)
        .unwrap();
        assert_eq!(
            &[0x18, 0x04, 0x01, 0x02, 0x03, 0x04, 0x50],
            &buffer[..count]
        );
    }
}
