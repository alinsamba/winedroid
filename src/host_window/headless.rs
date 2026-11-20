use std::env;
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{Result, WinedroidError};
use crate::frame_clock::FrameClock;
use crate::host_window::session::{
    self, ActiveClickHandlerGuard, ActiveRenderSurfaceGuard, NativeTextUpdate, SessionLoopHost,
    SessionSurfaceHost, FALLBACK_FRAMES_PER_SECOND, MAX_EVENT_WAKE_SECONDS, NANOS_PER_SECOND_F64,
};
use crate::host_window::{ActivityClickHandler, ActivityWindowRequest};

pub(in crate::host_window) fn headless_requested() -> Result<bool> {
    let value = match env::var("WINEDROID_HEADLESS") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => return Ok(false),
        Err(error) => {
            return Err(WinedroidError::with_source(
                "failed to read WINEDROID_HEADLESS",
                error,
            ));
        }
    };

    Ok(value == "1" || value.eq_ignore_ascii_case("true"))
}

struct HeadlessHost {
    render_on_redraw: bool,
}

impl SessionSurfaceHost for HeadlessHost {
    fn request_redraw(&self) {
        crate::render_metrics::record_host_redraw_request();
        if self.render_on_redraw {
            session::render_active_surface_for_metrics();
        }
    }

    fn refresh_native_text(
        &self,
        _handler: &ActivityClickHandler,
    ) -> Result<Vec<NativeTextUpdate>> {
        Ok(Vec::new())
    }
}

impl SessionLoopHost for HeadlessHost {
    fn pump_events(&self, wake_seconds: f64) -> Result<()> {
        if wake_seconds > 0.0 {
            thread::sleep(Duration::from_secs_f64(wake_seconds));
        }
        Ok(())
    }

    fn after_pump(&self) -> Result<()> {
        Ok(())
    }

    fn is_alive(&self) -> bool {
        true
    }
}

pub(in crate::host_window) fn run_headless_activity_session(
    request: &ActivityWindowRequest,
    click_handler: Option<ActivityClickHandler>,
) -> Result<()> {
    let autoclose_after = session::autoclose_after()?;
    let auto_click_targets = session::auto_click_targets()?;
    let _click_handler_guard = ActiveClickHandlerGuard::install(click_handler);
    let _render_surface_guard =
        ActiveRenderSurfaceGuard::install(session::initial_render_scroll_y()?);
    session::set_screenshot_dir(request.screenshots_dir.clone());
    session::install_render_surface_state(request)?;

    if crate::trace::enabled("window") {
        println!("Host headless session");
        println!("  package: {}", request.package_name);
        println!("  activity: {}", request.activity_name);
        println!("  size: {}x{}", request.width, request.height);
    }

    let interval_nanos = request
        .frame_interval_nanos
        .unwrap_or(NANOS_PER_SECOND_F64 as i64 / FALLBACK_FRAMES_PER_SECOND as i64);
    session::apply_runtime_frame_interval(interval_nanos)?;
    let host = HeadlessHost {
        render_on_redraw: false,
    };

    if let Some((
        script,
        log_path,
        opcode_log_path,
        usage_log_path,
        render_log_path,
        method_log_path,
    )) = session::automation_script_from_env()?
    {
        let host = HeadlessHost {
            render_on_redraw: render_log_path.is_some(),
        };
        return session::run_automation_script(
            &host,
            script,
            autoclose_after,
            interval_nanos,
            log_path,
            opcode_log_path,
            usage_log_path,
            render_log_path,
            method_log_path,
        );
    }

    session::perform_session_auto_clicks(&host, &auto_click_targets)?;

    let started_at = Instant::now();
    let wake_seconds = (interval_nanos as f64 / NANOS_PER_SECOND_F64).min(MAX_EVENT_WAKE_SECONDS);
    let mut frame_clock = FrameClock::new(interval_nanos);
    let mut wants_frames = true;

    while !session::should_autoclose(started_at, autoclose_after) {
        let this_wake_seconds = session::idle_wake_seconds(wake_seconds);
        thread::sleep(Duration::from_secs_f64(this_wake_seconds));

        if session::refresh_idle_render_surface() {
            wants_frames = true;
            frame_clock.reset();
        }

        if wants_frames {
            let host_now_nanos = started_at.elapsed().as_nanos() as i64;
            if let Some(pace) = frame_clock.poll(host_now_nanos) {
                wants_frames = session::drive_session_frame(&host, pace.skipped_intervals())?;
                if !wants_frames {
                    frame_clock.reset();
                }
            }
        }
    }

    Ok(())
}
