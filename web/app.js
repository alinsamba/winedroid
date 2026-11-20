const input = document.getElementById("apk-input");
const dropZone = document.getElementById("drop-zone");
const promptText = document.getElementById("prompt-text");
const loader = document.getElementById("loader");
const progressBar = document.getElementById("progress-bar");
const progressFill = document.getElementById("progress-fill");
const screen = document.getElementById("phone-screen");
const canvas = document.getElementById("winedroid-canvas");
const shell = screen.closest(".phone-shell");

const MAX_DISPLAY_DIMENSION = 4096;
const MAX_DISPLAY_PIXELS = 2_097_152;

let runtimeReady = false;
let runtimeFailure = null;
let runtimeAvailabilitySettled = false;
let resolveRuntimeAvailability;
let runtimeState = "idle";
let lastRuntimeError = "";
let revealTimer;
let displaySize;
let displaySizeFailure;

try {
  displaySize = requestedDisplaySize();
  applyDisplaySize(displaySize);
} catch (error) {
  displaySizeFailure = normalizeError(error);
}

const runtimeAvailability = new Promise(function (resolve) {
  resolveRuntimeAvailability = resolve;
});

globalThis.Module = {
  canvas,
  locateFile(path) {
    return `dist/${path}`;
  },
  print(message) {
    console.log(message);
  },
  printErr(message) {
    console.error(message);
  },
  onRuntimeInitialized() {
    runtimeReady = true;
    settleRuntimeAvailability();
  },
  onAbort(reason) {
    stopRuntime(reason || "The WebAssembly runtime aborted.");
  },
  onExit(code) {
    if (code !== 0) {
      stopRuntime(`The WebAssembly runtime exited with status ${code}.`);
    }
  },
};

input.addEventListener("change", function () {
  const file = input.files[0];
  if (file) {
    queueLaunch(file);
  }
});

for (const eventName of ["dragenter", "dragover"]) {
  dropZone.addEventListener(eventName, function (event) {
    event.preventDefault();
    if (acceptsFile()) {
      dropZone.dataset.dragging = "true";
    }
  });
}

for (const eventName of ["dragleave", "drop"]) {
  dropZone.addEventListener(eventName, function (event) {
    event.preventDefault();
    delete dropZone.dataset.dragging;
  });
}

dropZone.addEventListener("drop", function (event) {
  if (!acceptsFile()) {
    return;
  }
  const file = firstTransferredFile(event.dataTransfer);
  if (file) {
    queueLaunch(file);
  }
});

document.addEventListener("paste", function (event) {
  if (!acceptsFile()) {
    return;
  }
  const file = firstTransferredFile(event.clipboardData);
  if (file) {
    event.preventDefault();
    queueLaunch(file);
  }
});

document.addEventListener("winedroid-status", function (event) {
  if (!event.detail.failed) {
    return;
  }
  lastRuntimeError = event.detail.message;
  if (
    runtimeState === "starting"
    || runtimeState === "revealing"
    || runtimeState === "running"
  ) {
    showFailure(lastRuntimeError, false);
  }
});

document.addEventListener("winedroid-frame", function () {
  if (runtimeState !== "starting") {
    return;
  }
  runtimeState = "revealing";
  setProgress(100);
  revealTimer = setTimeout(showRunningApp, 260);
});

function acceptsFile() {
  return runtimeState === "idle";
}

function firstTransferredFile(transfer) {
  if (!transfer || !transfer.files || transfer.files.length === 0) {
    return null;
  }
  return transfer.files[0];
}

function queueLaunch(file) {
  if (!acceptsFile()) {
    return;
  }
  runtimeState = "loading";
  screen.dataset.state = "loading";
  promptText.textContent = "paste apk in here";
  promptText.removeAttribute("title");
  input.disabled = true;
  dropZone.setAttribute("aria-disabled", "true");
  dropZone.hidden = true;
  loader.hidden = false;
  canvas.hidden = true;
  setProgress(3);
  progressBar.focus({ preventScroll: true });
  void launch(file);
}

async function launch(file) {
  try {
    const bytes = await readApk(file);
    requireApkArchive(bytes);
    setProgress(48);
    await nextPaint();

    if (!runtimeReady) {
      setProgress(54);
      await runtimeAvailability;
    }
    if (runtimeFailure) {
      throw runtimeFailure;
    }

    Module.FS.mkdirTree("/winedroid-upload");
    Module.FS.writeFile("/winedroid-upload/app.apk", bytes);
    setProgress(66);
    await nextPaint();

    let launchResult = 0;
    try {
      launchResult = Module._winedroid_web_launch(
        displaySize.width,
        displaySize.height,
      );
    } catch (error) {
      if (error !== "unwind") {
        throw error;
      }
    }
    if (launchResult !== 0) {
      throw new Error(lastRuntimeError || "winedroid could not start this APK.");
    }

    runtimeState = "starting";
    setProgress(92);
  } catch (error) {
    showFailure(error, true);
  }
}

function readApk(file) {
  return new Promise(function (resolve, reject) {
    const reader = new FileReader();
    reader.addEventListener("progress", function (event) {
      if (event.lengthComputable) {
        const fraction = event.loaded / event.total;
        setProgress(5 + fraction * 38);
      }
    });
    reader.addEventListener("load", function () {
      resolve(new Uint8Array(reader.result));
    });
    reader.addEventListener("error", function () {
      reject(reader.error || new Error("The APK could not be read."));
    });
    reader.readAsArrayBuffer(file);
  });
}

function requireApkArchive(bytes) {
  if (bytes.length < 4 || bytes[0] !== 0x50 || bytes[1] !== 0x4b) {
    throw new Error("That file is not an APK archive.");
  }
}

function requestedDisplaySize() {
  const parameters = new URLSearchParams(globalThis.location.search);
  const configuredDisplays = parameters.getAll("display");
  if (configuredDisplays.length > 1) {
    throw new Error("The display query parameter may only be provided once.");
  }
  if (configuredDisplays.length === 1) {
    return parseDisplaySize(configuredDisplays[0], "display query parameter");
  }

  const defaultDisplay = `${screen.dataset.displayWidth}x${screen.dataset.displayHeight}`;
  return parseDisplaySize(defaultDisplay, "default browser display");
}

function parseDisplaySize(value, source) {
  const match = /^([1-9][0-9]*)x([1-9][0-9]*)$/.exec(value);
  if (!match) {
    throw new Error(`${source} must use WIDTHxHEIGHT with positive integers.`);
  }

  const width = Number(match[1]);
  const height = Number(match[2]);
  validateDisplayDimension(width, "width", source);
  validateDisplayDimension(height, "height", source);

  const pixels = width * height;
  if (pixels > MAX_DISPLAY_PIXELS) {
    throw new Error(
      `${source} must not exceed ${MAX_DISPLAY_PIXELS} logical pixels, got ${width}x${height}.`,
    );
  }

  return { width, height };
}

function validateDisplayDimension(value, name, source) {
  if (!Number.isSafeInteger(value) || value < 1 || value > MAX_DISPLAY_DIMENSION) {
    throw new Error(
      `${source} ${name} must be an integer from 1 through ${MAX_DISPLAY_DIMENSION}.`,
    );
  }
}

function applyDisplaySize(size) {
  const aspect = size.width / size.height;
  const heightAdjustment = 32 - aspect * 96;

  screen.dataset.displayWidth = String(size.width);
  screen.dataset.displayHeight = String(size.height);
  screen.style.aspectRatio = `${size.width} / ${size.height}`;
  shell.style.setProperty("--phone-height-limit-vh", `${aspect * 100}vh`);
  shell.style.setProperty("--phone-height-limit-svh", `${aspect * 100}svh`);
  shell.style.setProperty("--phone-height-adjustment", `${heightAdjustment}px`);
  canvas.width = size.width;
  canvas.height = size.height;
}

function nextPaint() {
  return new Promise(function (resolve) {
    requestAnimationFrame(function () {
      resolve();
    });
  });
}

function setProgress(value) {
  const progress = Math.max(0, Math.min(100, Math.round(value)));
  progressFill.style.width = `${progress}%`;
  progressBar.setAttribute("aria-valuenow", String(progress));
}

function showRunningApp() {
  loader.hidden = true;
  canvas.hidden = false;
  screen.dataset.state = "running";
  runtimeState = "running";
  canvas.focus();
}

function stopRuntime(error) {
  runtimeFailure = normalizeError(error);
  settleRuntimeAvailability();
  showFailure(runtimeFailure, false);
}

function settleRuntimeAvailability() {
  if (runtimeAvailabilitySettled) {
    return;
  }
  runtimeAvailabilitySettled = true;
  resolveRuntimeAvailability();
}

function showFailure(error, retry) {
  if (retry && runtimeState === "failed") {
    return;
  }
  const failure = normalizeError(error);
  console.error(failure.message);
  clearTimeout(revealTimer);
  runtimeState = retry ? "idle" : "failed";
  screen.dataset.state = "prompt";
  loader.hidden = true;
  canvas.hidden = true;
  dropZone.hidden = false;
  dropZone.setAttribute("aria-disabled", String(!retry));
  promptText.textContent = retry ? "couldn't open apk" : "winedroid stopped";
  promptText.setAttribute("title", failure.message);
  input.disabled = !retry;
  if (retry) {
    input.value = "";
  }
}

function normalizeError(error) {
  if (error instanceof Error) {
    return error;
  }
  return new Error(String(error));
}

if (displaySizeFailure) {
  stopRuntime(displaySizeFailure);
}
