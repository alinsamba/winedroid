import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const projectRoot = new URL("../", import.meta.url);
const loaderPath = new URL("web/dist/winedroid.js", projectRoot);
const wasmPath = new URL("web/dist/winedroid_web.wasm", projectRoot);
const displaySize = parseDisplaySize(process.env.WINEDROID_WEB_DISPLAY ?? "480x720");
const browserPixelRatio = Number(process.env.WINEDROID_WEB_PIXEL_RATIO ?? 2);
const canvasCssScale = 0.5;
const canvasRect = {
  left: 37,
  top: 19,
  width: displaySize.width * canvasCssScale,
  height: displaySize.height * canvasCssScale,
};
const canvasListeners = new Map();
let presentedPng;
let presentedFrames = 0;
let presentedFrameDetail;
let scheduledAnimationFrame;
let animationFrameSequence = 0;
const canvas = {
  dataset: {},
  hidden: true,
  addEventListener(type, listener) {
    canvasListeners.set(type, listener);
  },
  focus() {},
  getBoundingClientRect() {
    return canvasRect;
  },
  getContext() {
    return {
      clearRect() {},
      drawImage() {
        presentedFrames += 1;
      },
      imageSmoothingEnabled: true,
    };
  },
  setAttribute() {},
  setPointerCapture() {},
  tabIndex: 0,
  width: displaySize.width,
  height: displaySize.height,
};
const screen = {
  dataset: {
    displayWidth: String(displaySize.width),
    displayHeight: String(displaySize.height),
  },
  style: {},
};
const elements = new Map([
  ["runtime-status", { dataset: {}, textContent: "" }],
  ["runtime-detail", { dataset: {}, textContent: "" }],
  ["phone-screen", screen],
  ["winedroid-canvas", canvas],
]);

globalThis.devicePixelRatio = browserPixelRatio;
globalThis.CustomEvent = class CustomEvent {
  constructor(type, init) {
    this.type = type;
    this.detail = init?.detail;
  }
};
globalThis.document = {
  body: { dataset: {} },
  currentScript: { src: "http://127.0.0.1/web/dist/winedroid.js" },
  getElementById(id) {
    return elements.get(id) ?? null;
  },
  dispatchEvent(event) {
    if (event.type === "winedroid-frame") {
      presentedFrameDetail = event.detail;
    }
  },
};
globalThis.createImageBitmap = async (blob) => {
  presentedPng = new Uint8Array(await blob.arrayBuffer());
  return { close() {} };
};
globalThis.requestAnimationFrame = (callback) => {
  scheduledAnimationFrame = callback;
  animationFrameSequence += 1;
  return animationFrameSequence;
};
globalThis.cancelAnimationFrame = () => {
  scheduledAnimationFrame = undefined;
};

let runtimeReady;
const runtimeOutput = [];
const ready = new Promise((resolve) => {
  runtimeReady = resolve;
});
globalThis.Module = {
  wasmBinary: fs.readFileSync(wasmPath),
  print(message) {
    runtimeOutput.push(String(message));
  },
  printErr(message) {
    throw new Error(String(message));
  },
  onRuntimeInitialized() {
    runtimeReady();
  },
};

vm.runInThisContext(fs.readFileSync(loaderPath, "utf8"), {
  filename: loaderPath.pathname,
});

let timeout;
await Promise.race([
  ready,
  new Promise((_, reject) => {
    timeout = setTimeout(
      () => reject(new Error("WebAssembly runtime initialization timed out")),
      30_000,
    );
  }),
]);
clearTimeout(timeout);

assert.equal(typeof Module.FS, "object");
assert.equal(typeof Module._winedroid_web_launch, "function");
assert.equal(typeof Module._winedroid_web_pointer_down, "function");
assert.equal(typeof Module._winedroid_web_request_ui_dump, "function");
assert.equal(Module._winedroid_web_request_ui_dump(), 0);
Module.FS.mkdirTree("/winedroid-smoke");
Module.FS.writeFile("/winedroid-smoke/value", new Uint8Array([7, 8, 15]));
assert.deepEqual(
  Array.from(Module.FS.readFile("/winedroid-smoke/value")),
  [7, 8, 15],
);
assert.equal(elements.get("runtime-status").textContent, "Choose an APK to start");

const apkPath = process.argv[2];
if (apkPath) {
  const apk = fs.readFileSync(apkPath);
  Module.FS.mkdirTree("/winedroid-upload");
  Module.FS.writeFile("/winedroid-upload/app.apk", apk);
  try {
    Module._winedroid_web_launch(displaySize.width, displaySize.height);
  } catch (error) {
    if (error !== "unwind") {
      throw error;
    }
  }
  await waitFor(() => presentedFrames > 0);
  assert.deepEqual(Array.from(presentedPng.subarray(0, 8)), [137, 80, 78, 71, 13, 10, 26, 10]);
  const pixelScale = Math.max(1, Math.min(3, browserPixelRatio));
  const pixelWidth = Math.round(displaySize.width * pixelScale);
  const pixelHeight = Math.round(displaySize.height * pixelScale);
  assert.equal(screen.dataset.displayWidth, String(displaySize.width));
  assert.equal(screen.dataset.displayHeight, String(displaySize.height));
  assert.equal(screen.style.aspectRatio, `${displaySize.width} / ${displaySize.height}`);
  assert.equal(canvas.dataset.logicalWidth, String(displaySize.width));
  assert.equal(canvas.dataset.logicalHeight, String(displaySize.height));
  assert.equal(canvas.dataset.pixelWidth, String(pixelWidth));
  assert.equal(canvas.dataset.pixelHeight, String(pixelHeight));
  assert.equal(canvas.width, pixelWidth);
  assert.equal(canvas.height, pixelHeight);
  assert.equal(readPngU32(presentedPng, 16), pixelWidth);
  assert.equal(readPngU32(presentedPng, 20), pixelHeight);
  assert.deepEqual(presentedFrameDetail, {
    generation: Number(canvas.dataset.presented),
    logicalWidth: displaySize.width,
    logicalHeight: displaySize.height,
    pixelWidth,
    pixelHeight,
  });
  assert.match(elements.get("runtime-status").textContent, /^Running /);
  assert.equal(Module._winedroid_web_request_ui_dump(), 1);
  assert.match(Module.winedroidUiDump, /<hierarchy rotation="0">/);
  assert.match(
    Module.winedroidUiDump,
    new RegExp(`bounds="\\[0,0\\]\\[${displaySize.width},${displaySize.height}\\]"`),
  );
  assertPointerMapping();
  assertPixelWheelScaling();

  const tapTexts = process.env.WINEDROID_WEB_TAP_TEXTS?.split(",") ?? [];
  const framesPerTap = Number(process.env.WINEDROID_WEB_FRAMES_PER_TAP ?? 18);
  for (const text of tapTexts) {
    tapUiText(text);
    await runAnimationFrames(framesPerTap);
  }
  const expectedText = process.env.WINEDROID_WEB_EXPECT_TEXT;
  if (expectedText) {
    assert.equal(Module._winedroid_web_request_ui_dump(), 1);
    const containsExpectedText = Module.winedroidUiDump
      .split("\n")
      .some((line) => line.includes(` text="${expectedText}"`));
    if (!containsExpectedText) {
      console.error(runtimeOutput.join("\n"));
      console.error(Module.winedroidUiDump);
    }
    assert.ok(containsExpectedText, `UI dump does not contain text ${JSON.stringify(expectedText)}`);
  }
  await assertUnsupportedWheelStopsRuntime();
  console.log(`web APK smoke passed with ${presentedFrames} presented frame(s)`);
} else {
  console.log("web runtime boot and MEMFS smoke passed");
}

function tapUiText(text) {
  assert.equal(Module._winedroid_web_request_ui_dump(), 1);
  const line = Module.winedroidUiDump
    .split("\n")
    .find((candidate) => candidate.includes(` text="${text}"`) && candidate.includes(' clickable="true"'));
  assert.ok(line, `UI dump has no clickable text ${JSON.stringify(text)}`);
  const bounds = line.match(/bounds="\[(\d+),(\d+)\]\[(\d+),(\d+)\]"/);
  assert.ok(bounds, `UI node has no bounds for ${JSON.stringify(text)}`);
  const x = (Number(bounds[1]) + Number(bounds[3])) / 2;
  const y = (Number(bounds[2]) + Number(bounds[4])) / 2;
  dispatchPointerEvent("pointerdown", x, y);
  dispatchPointerEvent("pointerup", x, y);
}

function assertPointerMapping() {
  const originalPointerDown = Module._winedroid_web_pointer_down;
  let received;
  Module._winedroid_web_pointer_down = function (x, y) {
    received = { x, y };
  };

  const logicalX = displaySize.width * 0.25;
  const logicalY = displaySize.height * 0.75;
  dispatchPointerEvent("pointerdown", logicalX, logicalY);
  Module._winedroid_web_pointer_down = originalPointerDown;

  assert.ok(received);
  assert.equal(received.x, logicalX);
  assert.equal(received.y, logicalY);
}

function assertPixelWheelScaling() {
  const originalWheel = Module._winedroid_web_wheel;
  let received;
  Module._winedroid_web_wheel = function (deltaY, x, y) {
    received = { deltaY, x, y };
  };

  const logicalX = displaySize.width / 2;
  const logicalY = displaySize.height / 2;
  const event = browserPointEvent(logicalX, logicalY);
  event.deltaY = 12;
  event.deltaMode = 0;
  dispatchCanvasEvent("wheel", event);

  assert.ok(received);
  assert.equal(received.deltaY, 12 / canvasCssScale);
  assert.equal(received.x, logicalX);
  assert.equal(received.y, logicalY);

  Module._winedroid_web_wheel = originalWheel;
}

async function assertUnsupportedWheelStopsRuntime() {
  const originalPointerCancel = Module._winedroid_web_pointer_cancel;
  let pointerCanceled = false;
  Module._winedroid_web_pointer_cancel = function () {
    pointerCanceled = true;
    originalPointerCancel();
  };

  const logicalX = displaySize.width / 2;
  const logicalY = displaySize.height / 2;
  const unsupportedEvent = browserPointEvent(logicalX, logicalY);
  unsupportedEvent.deltaY = 1;
  unsupportedEvent.deltaMode = 1;
  const pendingFrame = scheduledAnimationFrame;
  assert.equal(typeof pendingFrame, "function");
  dispatchCanvasEvent("wheel", unsupportedEvent);
  Module._winedroid_web_pointer_cancel = originalPointerCancel;

  assert.equal(pointerCanceled, true);
  assert.equal(document.body.dataset.runtime, "stopped");
  assert.equal(elements.get("runtime-status").textContent, "Stopped");
  assert.match(
    elements.get("runtime-detail").textContent,
    /wheel input requires pixel-mode deltas/,
  );

  scheduledAnimationFrame = undefined;
  pendingFrame(performance.now());
  await Promise.resolve();
  assert.equal(scheduledAnimationFrame, undefined);
}

function dispatchPointerEvent(type, logicalX, logicalY) {
  const event = browserPointEvent(logicalX, logicalY);
  event.pointerId = 1;
  dispatchCanvasEvent(type, event);
}

function browserPointEvent(logicalX, logicalY) {
  return {
    clientX: canvasRect.left + logicalX * canvasRect.width / displaySize.width,
    clientY: canvasRect.top + logicalY * canvasRect.height / displaySize.height,
    defaultPrevented: false,
    preventDefault() {
      this.defaultPrevented = true;
    },
  };
}

function dispatchCanvasEvent(type, event) {
  const listener = canvasListeners.get(type);
  assert.equal(typeof listener, "function", `canvas has no ${type} listener`);
  listener(event);
  assert.equal(event.defaultPrevented, true, `${type} was not prevented`);
}

function parseDisplaySize(value) {
  const match = /^([1-9][0-9]*)x([1-9][0-9]*)$/.exec(value);
  assert.ok(match, `invalid WINEDROID_WEB_DISPLAY ${JSON.stringify(value)}`);
  return {
    width: Number(match[1]),
    height: Number(match[2]),
  };
}

function readPngU32(bytes, offset) {
  return new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(offset);
}

async function waitFor(predicate) {
  const deadline = performance.now() + 30_000;
  while (!predicate()) {
    if (performance.now() >= deadline) {
      throw new Error("Web APK frame timed out");
    }
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}

async function runAnimationFrames(count) {
  for (let index = 0; index < count; index += 1) {
    const callback = scheduledAnimationFrame;
    assert.equal(typeof callback, "function", "runtime did not schedule an animation frame");
    scheduledAnimationFrame = undefined;
    await new Promise((resolve) => setTimeout(resolve, 18));
    callback(performance.now());
    await Promise.resolve();
  }
}
