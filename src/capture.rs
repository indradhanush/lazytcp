use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Idle,
    Running,
}

#[derive(Debug)]
pub struct CaptureError {
    message: String,
}

impl CaptureError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for CaptureError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CaptureError {}

pub trait CaptureBackend {
    fn start(&mut self) -> Result<(), CaptureError>;
    fn stop(&mut self) -> Result<(), CaptureError>;
    fn state(&self) -> CaptureState;
}

pub struct NoopCaptureBackend {
    state: CaptureState,
}

impl NoopCaptureBackend {
    pub fn new() -> Self {
        Self {
            state: CaptureState::Idle,
        }
    }
}

impl Default for NoopCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureBackend for NoopCaptureBackend {
    fn start(&mut self) -> Result<(), CaptureError> {
        if self.state == CaptureState::Running {
            return Err(CaptureError::new("capture is already running"));
        }
        self.state = CaptureState::Running;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), CaptureError> {
        if self.state == CaptureState::Idle {
            return Err(CaptureError::new("capture is already stopped"));
        }
        self.state = CaptureState::Idle;
        Ok(())
    }

    fn state(&self) -> CaptureState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::{CaptureBackend, CaptureState, NoopCaptureBackend};

    #[test]
    fn start_transitions_backend_to_running() {
        let mut backend = NoopCaptureBackend::new();

        backend.start().expect("start should succeed from idle");

        assert_eq!(backend.state(), CaptureState::Running);
    }

    #[test]
    fn start_returns_error_when_already_running() {
        let mut backend = NoopCaptureBackend::new();
        backend.start().expect("first start should succeed");

        let error = backend.start().expect_err("second start should fail");

        assert_eq!(error.to_string(), "capture is already running");
        assert_eq!(backend.state(), CaptureState::Running);
    }

    #[test]
    fn stop_transitions_backend_to_idle_after_start() {
        let mut backend = NoopCaptureBackend::new();
        backend.start().expect("start should succeed");

        backend.stop().expect("stop should succeed from running");

        assert_eq!(backend.state(), CaptureState::Idle);
    }

    #[test]
    fn stop_returns_error_when_already_idle() {
        let mut backend = NoopCaptureBackend::new();

        let error = backend.stop().expect_err("stop should fail from idle");

        assert_eq!(error.to_string(), "capture is already stopped");
        assert_eq!(backend.state(), CaptureState::Idle);
    }
}
