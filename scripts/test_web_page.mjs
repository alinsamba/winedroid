import assert from "node:assert/strict";
import fs from "node:fs";
import { performance } from "node:perf_hooks";
import vm from "node:vm";

const projectRoot = new URL("../", import.meta.url);
const html = fs.readFileSync(new URL("web/index.html", projectRoot), "utf8");
const appSource = fs.readFileSync(new URL("web/app.js", projectRoot), "utf8");
const screenTag = openingTagWithId(html, "phone-screen");
const defaultWidth = Number(attributeValue(screenTag, "data-display-width"));
const defaultHeight = Number(attributeValue(screenTag, "data-display-height"));

function createPage(search) {
  const shell = new FakeElement("phone-shell");
  const screen = new FakeElement("phone-screen");
  const canvas = new FakeElement("winedroid-canvas");
  const input = new FakeElement("apk-input");
  const dropZone = new FakeElement("drop-zone");
  const prompt = new FakeElement("prompt-text");
  const loader = new FakeElement("loader");
  const progressBar = new FakeElement("progress-bar");
  const progressFill = new FakeElement("progress-fill");
  screen.dataset.displayWidth = String(defaultWidth);
  screen.dataset.displayHeight = String(defaultHeight);
  screen.closest = function (selector) {
    assert.equal(selector, ".phone-shell");
    return shell;
  };
  input.files = [];
  loader.hidden = true;
  canvas.hidden = true;

  const elements = new Map([
    ["apk-input", input],
    ["drop-zone", dropZone],
    ["prompt-text", prompt],
    ["loader", loader],
    ["progress-bar", progressBar],
    ["progress-fill", progressFill],
    ["phone-screen", screen],
    ["winedroid-canvas", canvas],
  ]);
  const document = new FakeEventTarget();
  document.body = new FakeElement("body");
  document.getElementById = function (id) {
    return elements.get(id) ?? null;
  };

  let animationFrameId = 0;
  const context = vm.createContext({
    Array,
    CustomEvent: class CustomEvent {
      constructor(type, init) {
        this.type = type;
        this.detail = init?.detail;
      }
    },
    FileReader: FakeFileReader,
    Math,
    Number,
    Promise,
    Uint8Array,
    URLSearchParams,
    clearTimeout,
    console: {
      error() {},
      log() {},
    },
    document,
    location: { search },
    requestAnimationFrame(callback) {
      animationFrameId += 1;
      queueMicrotask(function () {
        callback(performance.now());
      });
      return animationFrameId;
    },
    setTimeout,
  });
  vm.runInContext(appSource, context, { filename: "web/app.js" });
  return { canvas, context, dropZone, input, loader, progressBar, prompt, screen, shell };
}

class FakeEventTarget {
  constructor() {
    this.listeners = new Map();
  }

  addEventListener(type, listener) {
    const listeners = this.listeners.get(type) ?? [];
    listeners.push(listener);
    this.listeners.set(type, listeners);
  }

  dispatchEvent(event) {
    const listeners = this.listeners.get(event.type) ?? [];
    for (const listener of listeners) {
      listener.call(this, event);
    }
    return !event.defaultPrevented;
  }
}

class FakeElement extends FakeEventTarget {
  constructor(id) {
    super();
    this.id = id;
    this.attributes = new Map();
    this.dataset = {};
    this.disabled = false;
    this.hidden = false;
    this.style = new FakeStyle();
    this.textContent = "";
    this.value = "";
  }

  focus() {
    this.focused = true;
  }

  removeAttribute(name) {
    this.attributes.delete(name);
  }

  setAttribute(name, value) {
    this.attributes.set(name, String(value));
  }
}

class FakeStyle {
  constructor() {
    this.properties = new Map();
  }

  getPropertyValue(name) {
    return this.properties.get(name) ?? "";
  }

  setProperty(name, value) {
    this.properties.set(name, String(value));
  }
}

class FakeFileReader extends FakeEventTarget {
  readAsArrayBuffer(file) {
    this.result = file.bytes.buffer.slice(
      file.bytes.byteOffset,
      file.bytes.byteOffset + file.bytes.byteLength,
    );
    const reader = this;
    queueMicrotask(function () {
      reader.dispatchEvent({ type: "load" });
    });
  }
}

function openingTagWithId(source, id) {
  const escapedId = id.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = new RegExp(`<[^>]+\\bid=["']${escapedId}["'][^>]*>`).exec(source);
  assert.ok(match, `HTML has no element with id ${JSON.stringify(id)}`);
  return match[0];
}

function attributeValue(tag, name) {
  const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = new RegExp(`\\b${escapedName}=["']([^"']+)["']`).exec(tag);
  assert.ok(match, `HTML element has no ${name} attribute`);
  return match[1];
}

async function waitFor(predicate) {
  const deadline = performance.now() + 1_000;
  while (!predicate()) {
    if (performance.now() >= deadline) {
      throw new Error("web page launch call timed out");
    }
    await new Promise(function (resolve) {
      setTimeout(resolve, 1);
    });
  }
}

async function launchPage(page) {
  const launchArguments = [];
  const writtenFiles = [];
  page.context.Module.FS = {
    mkdirTree(path) {
      assert.equal(path, "/winedroid-upload");
    },
    writeFile(path, bytes) {
      writtenFiles.push({ path, bytes: Array.from(bytes) });
    },
  };
  page.context.Module._winedroid_web_launch = function (width, height) {
    launchArguments.push({ width, height });
    return 0;
  };
  page.context.Module.onRuntimeInitialized();
  page.input.files = [{ bytes: new Uint8Array([0x50, 0x4b, 0x03, 0x04]) }];
  page.input.dispatchEvent({ type: "change" });
  await waitFor(function () {
    return launchArguments.length === 1;
  });
  return { launchArguments, writtenFiles };
}

async function main() {
  assert.equal(defaultWidth, 420);
  assert.equal(defaultHeight, 910);

  const defaultPage = createPage("");
  assert.equal(defaultPage.screen.dataset.displayWidth, "420");
  assert.equal(defaultPage.screen.dataset.displayHeight, "910");
  assert.equal(defaultPage.screen.style.aspectRatio, "420 / 910");
  assert.equal(defaultPage.canvas.width, 420);
  assert.equal(defaultPage.canvas.height, 910);
  const defaultLaunch = await launchPage(defaultPage);
  assert.deepEqual(defaultLaunch.launchArguments, [{ width: 420, height: 910 }]);

  const customPage = createPage("?display=360x800");
  assert.equal(customPage.screen.dataset.displayWidth, "360");
  assert.equal(customPage.screen.dataset.displayHeight, "800");
  assert.equal(customPage.screen.style.aspectRatio, "360 / 800");
  assert.equal(customPage.canvas.width, 360);
  assert.equal(customPage.canvas.height, 800);
  assert.equal(customPage.shell.style.getPropertyValue("--phone-height-limit-vh"), "45vh");
  assert.equal(customPage.shell.style.getPropertyValue("--phone-height-limit-svh"), "45svh");

  const exactLimit = vm.runInContext(
    'parseDisplaySize("4096x512", "test display")',
    customPage.context,
  );
  assert.equal(exactLimit.width, 4096);
  assert.equal(exactLimit.height, 512);
  assert.throws(
    function () {
      vm.runInContext('parseDisplaySize("4096x513", "test display")', customPage.context);
    },
    /must not exceed 2097152 logical pixels/,
  );
  assert.throws(
    function () {
      vm.runInContext('parseDisplaySize("4097x1", "test display")', customPage.context);
    },
    /width must be an integer from 1 through 4096/,
  );
  for (const invalid of ["0x910", "420x0", "0420x910", "420X910", "420.5x910"]) {
    assert.throws(
      function () {
        vm.runInContext(
          `parseDisplaySize(${JSON.stringify(invalid)}, "test display")`,
          customPage.context,
        );
      },
      /must use WIDTHxHEIGHT with positive integers/,
    );
  }

  const duplicatePage = createPage("?display=360x800&display=420x910");
  assert.equal(duplicatePage.prompt.textContent, "winedroid stopped");
  assert.match(duplicatePage.prompt.attributes.get("title"), /may only be provided once/);
  assert.equal(duplicatePage.input.disabled, true);

  const customLaunch = await launchPage(customPage);
  assert.deepEqual(customLaunch.launchArguments, [{ width: 360, height: 800 }]);
  assert.deepEqual(customLaunch.writtenFiles, [{
    path: "/winedroid-upload/app.apk",
    bytes: [0x50, 0x4b, 0x03, 0x04],
  }]);

  console.log("web page display configuration smoke passed");
}

await main();
