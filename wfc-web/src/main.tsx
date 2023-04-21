import { render } from "solid-js/web";
import { createSignal, Show, onMount } from "solid-js";
import { createStore } from "solid-js/store";
// TODO: figure out how to use wasm bindgen to generate wfc-web.d.ts before webpack webpacks
import type {
  WfcController,
  WfcData,
  WfcWindow,
  WfcWebBuilder,
  InitOutput,
} from "./wfc-web.d.ts";
import type * as WfcNamespace from "./wfc-web.d.ts";

// TODO: figure out why this works
type Wfc = typeof WfcNamespace;

function PlayControls(props: { controller: WfcController }) {
  // NOTE: playing implies pause icon and vice versa
  const [playing, setPlaying] = createSignal(false);

  const toggle = () => {
    props.controller.toggle_playing();
    setPlaying(!playing());
  };

  const PauseIcon = (
    <img
      id="PAUSE_ICON"
      class="w-10 h-10"
      src="https://img.icons8.com/ios-filled/50/FFFFFF/pause--v1.png"
    />
  );

  const PlayIcon = (
    <img
      id="PLAY_ICON"
      class="w-10 h-10"
      src="https://img.icons8.com/ios-filled/50/FFFFFF/play--v1.png"
    />
  );

  const playing_bg = "bg-green-500";
  const pause_bg = "bg-red-500";

  return (
    <div
      id="play-controls"
      class="z-50 bottom-0 flex flex-row justify-between my-2"
    >
      <div id="left-play-controls">
        <button
          class={`${
            playing() ? pause_bg : playing_bg
          } block rounded-md px-4 py-2`}
          onClick={toggle}
          id="play-pause"
        >
          <Show when={playing()} fallback={PlayIcon}>
            {PauseIcon}
          </Show>
        </button>
      </div>
      <div id="right-play-controls">
        <button class="bg-blue-500 block rounded-md">
          <img
            class="w-14 h-14"
            src="https://img.icons8.com/external-others-inmotus-design/67/FFFFFF/external-Reload-round-icons-others-inmotus-design-6.png"
          />
        </button>
      </div>
    </div>
  );
}

async function wangTileBytes(wang_tile: string): Promise<Uint8Array> {
  const path = `./assets/wang/${wang_tile}.png`;
  const bytes: Uint8Array = await fetch(path)
    .then((response) => response.blob())
    .then((blob) => {
      const reader = new FileReader();
      reader.readAsArrayBuffer(blob);
      return new Promise((resolve, reject) => {
        reader.onloadend = function () {
          const u8Array = new Uint8Array(reader.result as ArrayBufferLike);
          resolve(u8Array);
        };
        reader.onerror = function () {
          reject(reader.error);
        };
      });
    });
  return bytes;
}

interface PlayerSettings {
  tile_size: number;
  output_size: { x: number; y: number };
  wang: boolean;
}

function PresetSelector(props: {
  wfc: Wfc;
  controller: WfcController;
  setSettings: (arg0: PlayerSettings) => void;
}) {
  const { wfc, controller, setSettings } = props;
  type PresetImage = "dual" | "celtic";
  const [preset, setPresetSignal]: [
    () => PresetImage,
    (p: PresetImage) => void
  ] = createSignal("dual");
  const presetSettings: { [Key in PresetImage]: PlayerSettings } = {
    dual: {
      tile_size: 32,
      output_size: { x: 256, y: 256 },
      wang: true,
    },
    celtic: {
      tile_size: 32,
      output_size: { x: 256, y: 256 },
      wang: true,
    },
  };

  async function loadPreset(value: PresetImage) {
    console.log("Selected preset:", value);
    setPresetSignal(value);
    const bytes = await wangTileBytes(value);
    const settings = presetSettings[value];
    let wfcData: WfcData = wfc.WfcWebBuilder.new_from_image_bytes(bytes)
      .with_tile_size(settings.tile_size)
      .with_output_dimensions(settings.output_size.x, settings.output_size.y)
      .wang()
      .build();
    setSettings(settings);
    controller.load_wfc(wfcData);
  }
  onMount(() => {
    loadPreset(preset());
  });
  // TODO: add button toggle to set whether to load preset settings
  // along with image (so that users can maintain their own settings)
  return (
    <>
      <label for="presets">Presets</label>
      <select
        id="presets"
        class="border-2 text-white bg-transparent rounded-sm p-1"
        onChange={(e) => loadPreset(e.target!.value as PresetImage)}
        value={preset()}
      >
        <option value="dual">Dual</option>
        <option value="celtic">Celtic</option>
      </select>
    </>
  );
}
function ConfigMenu(props: { wfc: Wfc; controller: WfcController }) {
  const { wfc, controller } = props;
  const [settings, setSettings] = createStore({
    tile_size: 2,
    output_size: { x: 256, y: 256 },
  });
  return (
    <div
      id="config-menu"
      class="flex shrink flex-row lg:flex-col lg:ml-4 my-4 rounded-md border-2 text-white p-2"
    >
      <PresetSelector
        wfc={wfc}
        controller={controller}
        setSettings={setSettings}
      />
      <div class="border-b border-2 border-white my-4 lg:w-full"></div>
      <span>
        <b>Preprocessor Settings</b>
      </span>
      <div class="flex-row">
        <span class="mr-1">Tile Size:</span>
        <input
          type="number"
          class="w-10 bg-transparent border-1"
          value={settings.tile_size}
        ></input>
      </div>
      <div class="border-b border-2 border-white my-4 lg:w-full"></div>
      <span>
        <b>Model Settings</b>
      </span>
      <div class="flex-row">
        <span class="mr-1">Output Size:</span>
        <input
          type="number"
          class="w-12 bg-transparent border-1"
          value={settings.output_size.x}
        ></input>
        <span class="mx-1">x</span>
        <input
          type="number"
          class="w-12 bg-transparent border-1"
          value={settings.output_size.y}
        ></input>
      </div>
    </div>
  );
}

declare global {
  interface Window {
    // script? in Trunk.toml initializes wasm and adds binding
    // to window. Wasm initialization takes a while though so it will
    // be undefined until the promise resolves
    wasm: Wfc | undefined;
  }
}

async function init() {
  // Wait for wasm module to load and be added to window
  const wfc: Wfc = await new Promise((resolve) => {
    console.log("waiting for wasm");
    const intervalId = setInterval(() => {
      console.log("checking for wasm");
      if (window.wasm !== undefined) {
        clearInterval(intervalId);
        resolve(window.wasm);
      }
    }, 100);
  });
  console.log("wasm loaded");
  let display = await wfc.WfcWindow.new();
  const controller = wfc.WfcController.init(display);

  window.addEventListener("resize", (_e) => {
    let canvas_container = document.getElementById("canvas-container");
    if (!canvas_container) {
      console.error("canvas-container element not found!");
      return;
    }
    let r = window.devicePixelRatio || 1;
    let w = canvas_container.offsetWidth;
    let h = canvas_container.offsetHeight;
    let styles = window.getComputedStyle(canvas_container);
    w =
      w -
      parseFloat(styles.paddingLeft) -
      parseFloat(styles.paddingRight) -
      parseFloat(styles.marginLeft) -
      parseFloat(styles.marginRight);
    h =
      h -
      parseFloat(styles.paddingTop) -
      parseFloat(styles.paddingBottom) -
      parseFloat(styles.marginTop) -
      parseFloat(styles.marginBottom);
    // NOTE: scaling by r converts w,h into
    // winit PhysicalSize units
    w = Math.round(w * r);
    h = Math.round(h * r);
    controller.resize_canvas(w, h);
  });
  render(
    () => <PlayControls controller={controller} />,
    document.getElementById("player")!
  );
  render(
    () => <ConfigMenu wfc={wfc} controller={controller} />,
    document.getElementById("player+menu")!
  );

  console.log("Starting event-loop");
  // NOTE: this is blocking! nothing after this will run!
  display.start_event_loop();
}

init();
