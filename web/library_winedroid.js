mergeInto(LibraryManager.library, {
  $WinedroidWebHost: {
    canvas: null,
    logicalWidth: 0,
    logicalHeight: 0,
    presentation: 0,
    running: false,

    module() {
      return typeof Module === "object" ? Module : globalThis.Module;
    },

    surfaceRect() {
      const rect = this.canvas.getBoundingClientRect();
      if (
        !Number.isFinite(rect.left)
        || !Number.isFinite(rect.top)
        || !Number.isFinite(rect.width)
        || !Number.isFinite(rect.height)
        || rect.width <= 0
        || rect.height <= 0
      ) {
        throw new Error("winedroid canvas has invalid browser geometry");
      }
      return rect;
    },

    point(event, rect) {
      if (!Number.isFinite(event.clientX) || !Number.isFinite(event.clientY)) {
        throw new Error("winedroid pointer coordinates are invalid");
      }
      return {
        x: (event.clientX - rect.left) * this.logicalWidth / rect.width,
        y: (event.clientY - rect.top) * this.logicalHeight / rect.height,
      };
    },

    wheelDelta(event, rect) {
      if (!Number.isFinite(event.deltaY)) {
        throw new Error("winedroid wheel delta is invalid");
      }

      if (event.deltaMode !== 0) {
        throw new Error(
          `winedroid wheel input requires pixel-mode deltas, got mode ${event.deltaMode}`,
        );
      }

      return event.deltaY * this.logicalHeight / rect.height;
    },

    setStatus(message, failed) {
      const status = document.getElementById("runtime-status");
      const detail = document.getElementById("runtime-detail");
      if (status) {
        status.textContent = failed ? "Stopped" : message;
        status.dataset.state = failed ? "error" : "ready";
      }
      if (detail && failed) {
        detail.textContent = message;
      }
      document.dispatchEvent(new CustomEvent("winedroid-status", {
        detail: { message, failed },
      }));
    },

    stop(message) {
      const wasRunning = this.running;
      this.running = false;
      this.presentation += 1;
      document.body.dataset.runtime = "stopped";
      if (wasRunning) {
        _emscripten_cancel_main_loop();
        this.module()._winedroid_web_pointer_cancel();
      }
      this.setStatus(message, true);
    },
  },

  winedroid_web_install__deps: ["$WinedroidWebHost"],
  winedroid_web_install(width, height) {
    const host = WinedroidWebHost;
    const canvas = document.getElementById("winedroid-canvas");
    const screen = document.getElementById("phone-screen");
    if (!canvas) {
      throw new Error("winedroid canvas was not found");
    }
    if (!screen) {
      throw new Error("winedroid phone screen was not found");
    }
    if (
      screen.dataset.displayWidth !== String(width)
      || screen.dataset.displayHeight !== String(height)
    ) {
      throw new Error("winedroid browser and runtime display sizes do not match");
    }
    host.canvas = canvas;
    host.logicalWidth = width;
    host.logicalHeight = height;
    host.running = true;
    screen.style.aspectRatio = `${width} / ${height}`;
    canvas.dataset.logicalWidth = String(width);
    canvas.dataset.logicalHeight = String(height);
    canvas.hidden = false;
    canvas.tabIndex = 0;
    canvas.setAttribute("aria-label", "Running Android application");

    canvas.addEventListener("pointerdown", (event) => {
      const rect = host.surfaceRect();
      const point = host.point(event, rect);
      canvas.setPointerCapture(event.pointerId);
      canvas.focus();
      host.module()._winedroid_web_pointer_down(point.x, point.y, performance.now());
      event.preventDefault();
    });
    canvas.addEventListener("pointermove", (event) => {
      const rect = host.surfaceRect();
      const point = host.point(event, rect);
      host.module()._winedroid_web_pointer_move(point.x, point.y);
    });
    canvas.addEventListener("pointerup", (event) => {
      const rect = host.surfaceRect();
      const point = host.point(event, rect);
      host.module()._winedroid_web_pointer_up(point.x, point.y, performance.now());
      event.preventDefault();
    });
    canvas.addEventListener("pointercancel", () => {
      host.module()._winedroid_web_pointer_cancel();
    });
    canvas.addEventListener("wheel", (event) => {
      try {
        const rect = host.surfaceRect();
        const point = host.point(event, rect);
        const deltaY = host.wheelDelta(event, rect);
        host.module()._winedroid_web_wheel(deltaY, point.x, point.y);
      } catch (error) {
        host.stop(String(error));
      }
      event.preventDefault();
    }, { passive: false });
    canvas.addEventListener("keydown", (event) => {
      const keys = {
        Enter: 1,
        Backspace: 2,
        Delete: 3,
        ArrowLeft: 4,
        ArrowRight: 5,
        Home: 6,
        End: 7,
      };
      const key = keys[event.key] || 0;
      const characters = Array.from(event.key);
      const codePoint = key === 0 && characters.length === 1
        ? characters[0].codePointAt(0)
        : 0;
      if (key === 0 && codePoint === 0) {
        return;
      }
      host.module()._winedroid_web_key(key, codePoint);
      event.preventDefault();
    });

    document.body.dataset.runtime = "running";
  },

  winedroid_web_present_png__deps: ["$WinedroidWebHost"],
  winedroid_web_present_png(data, length, pixelWidth, pixelHeight) {
    const host = WinedroidWebHost;
    const bytes = HEAPU8.slice(data, data + length);
    const generation = ++host.presentation;
    createImageBitmap(new Blob([bytes], { type: "image/png" })).then((bitmap) => {
      if (generation !== host.presentation || !host.canvas) {
        bitmap.close();
        return;
      }
      const context = host.canvas.getContext("2d", { alpha: false });
      host.canvas.width = pixelWidth;
      host.canvas.height = pixelHeight;
      context.imageSmoothingEnabled = true;
      context.clearRect(0, 0, pixelWidth, pixelHeight);
      context.drawImage(bitmap, 0, 0, pixelWidth, pixelHeight);
      bitmap.close();
      host.canvas.dataset.presented = String(generation);
      host.canvas.dataset.pixelWidth = String(pixelWidth);
      host.canvas.dataset.pixelHeight = String(pixelHeight);
      document.dispatchEvent(new CustomEvent("winedroid-frame", {
        detail: {
          generation,
          logicalWidth: host.logicalWidth,
          logicalHeight: host.logicalHeight,
          pixelWidth,
          pixelHeight,
        },
      }));
    }).catch((error) => host.stop(String(error)));
  },

  winedroid_web_device_pixel_ratio() {
    return globalThis.devicePixelRatio || 1;
  },

  winedroid_web_status__deps: ["$WinedroidWebHost"],
  winedroid_web_status(data, length) {
    WinedroidWebHost.setStatus(UTF8ToString(data, length), false);
  },

  winedroid_web_error__deps: ["$WinedroidWebHost"],
  winedroid_web_error(data, length) {
    WinedroidWebHost.stop(UTF8ToString(data, length));
  },

  winedroid_web_ui_dump__deps: ["$WinedroidWebHost"],
  winedroid_web_ui_dump(data, length) {
    const dump = UTF8ToString(data, length);
    WinedroidWebHost.module().winedroidUiDump = dump;
    document.dispatchEvent(new CustomEvent("winedroid-ui-dump", {
      detail: { dump },
    }));
  },

  winedroid_web_open_url(data, length) {
    const url = UTF8ToString(data, length);
    const opened = window.open("", "_blank");
    if (!opened) {
      return 0;
    }
    opened.opener = null;
    opened.location.replace(url);
    return 1;
  },
});
