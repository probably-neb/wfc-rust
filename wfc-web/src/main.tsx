import { render } from "solid-js/web";
import { createSignal, Show, createEffect, Accessor, Setter } from "solid-js";
import { createStore } from "solid-js/store";
import type { WfcController, WfcData } from "./wfc-web.d.ts";

import type * as WfcNamespace from "./wfc-web.d.ts";
type Wfc = typeof WfcNamespace;

type ReloadFunc = () => Promise<void>;

interface PlayerSettings {
  tile_size: number;
  output_size: { x: number; y: number };
  wang: boolean;
  image: PresetImage;
}


const DEFAULT_PRESET: PresetImage = "dual";

const PRESET_SETTINGS: Record<string, PlayerSettings> = {
  dual: {
    tile_size: 32,
    output_size: { x: 256, y: 256 },
    wang: true,
    image: "dual",
  },
  celtic: {
    tile_size: 32,
    output_size: { x: 256, y: 256 },
    wang: true,
    image: "celtic",
  },
  corners: {
        tile_size: 3,
        output_size: { x: 256, y: 256 },
        wang: false,
        image: "corners",
    }
};

type PresetImage = keyof typeof PRESET_SETTINGS;

function PlayControls(props: {
  controller: Accessor<WfcController | undefined>;
  reload: Accessor<ReloadFunc>;
}) {
  // NOTE: playing implies pause icon and vice versa
  const [playing, setPlaying] = createSignal(false);

  const toggle = () => {
    setPlaying(!playing());
    props.controller()?.toggle_playing();
  };
  const reload = () => {
    setPlaying(false);
    props.reload()();
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

  return (
    <div
      id="play-controls"
      class="z-50 bottom-0 flex flex-row justify-between my-2"
    >
      <div id="left-play-controls">
        <button
          class={`${
            playing() ? "bg-red-500" : "bg-green-500" // red if playing for pause button and green if paused for play button
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
        <button
          class="bg-blue-500 block rounded-md"
          id="reload-button"
          onClick={reload}
        >
          <img
            class="w-14 h-14"
            src="https://img.icons8.com/external-others-inmotus-design/67/FFFFFF/external-Reload-round-icons-others-inmotus-design-6.png"
          />
        </button>
      </div>
    </div>
  );
}

async function loadPresetImage(preset_name: PresetImage): Promise<Uint8Array> {
  const preset = PRESET_SETTINGS[preset_name];
  const path = `./assets/${preset.wang ? "wang" : "non-wang"}/${preset.image}.png`;
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

function buildWfc(
  wfc: Wfc,
  settings: PlayerSettings,
  imageBytes: Uint8Array
): WfcData {
  const wfcData: WfcData = wfc.WfcWebBuilder.new_from_image_bytes(imageBytes)
    .with_tile_size(settings.tile_size)
    .with_output_dimensions(settings.output_size.x, settings.output_size.y)
    .wang(settings.wang)
    .build();
  return wfcData;
}

function PresetSelector(props: {
  wfc: Accessor<Wfc | undefined>;
  controller: Accessor<WfcController | undefined>;
  loading: Accessor<boolean>;
  setSettings: (arg0: PlayerSettings) => void;
}) {
  const { wfc, controller, loading } = props;
  const [preset, setPreset] = createSignal<PresetImage | null>(null);
  createEffect(async () => {
    if (preset()) {
      const settings = PRESET_SETTINGS[preset()!];
      props.setSettings(settings);

      if (!loading()) {
        console.log("Selected preset:", preset()!);
        // TODO: put image loading in a createResource()
        const bytes = await loadPresetImage(settings.image);
        const wfcData = buildWfc(wfc()!, settings, bytes);
        controller()!.load_wfc(wfcData);
      } else {
        console.log("wfc not loaded");
      }
    }
  });
  // set preset once wasm is done loading
  createEffect(() => {
    if (!loading()) {
      setPreset(DEFAULT_PRESET);
    }
  });
  // TODO: add button toggle to set whether to load preset settings
  // along with image (so that users can maintain their own settings)
  return (
    <>
      <label for="presets">Presets</label>
      <select
        id="presets"
        class="border-2 text-white bg-transparent rounded-sm p-1"
        onChange={(e) => setPreset(e.target!.value as PresetImage)}
        value={preset() || undefined}
      >
        {Object.keys(PRESET_SETTINGS).map((preset) => (
            <option value={preset}>{preset}</option>
        ))}
      </select>
    </>
  );
}
function PlayerSettingsMenu(props: {
  wfc: Accessor<Wfc | undefined>;
  controller: Accessor<WfcController | undefined>;
  loading: Accessor<boolean>;
  setReloadFunc: Setter<ReloadFunc>;
}) {
  const [settings, setSettings] = createStore<PlayerSettings>(
    PRESET_SETTINGS[DEFAULT_PRESET]
  );
  async function loadWfc() {
    if (!props.loading()) {
      console.log("loading:", settings.image);
      const bytes = await loadPresetImage(settings.image);
      const wfcData = buildWfc(props.wfc()!, settings, bytes);
      props.controller()!.load_wfc(wfcData);
    }
  }
  props.setReloadFunc((_prev) => loadWfc);
  // TODO: resize text in input field text boxes on overflow
  // TODO: make `input` component that does ^ and other repeated logic below
  return (
    <div
      id="config-menu"
      class="flex shrink flex-row lg:flex-col lg:ml-4 my-4 rounded-md border-2 text-white p-2"
    >
      <PresetSelector
        wfc={props.wfc}
        controller={props.controller}
        loading={props.loading}
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
          class="w-10 bg-transparent border-2 truncate hover:whitespace-normal"
          value={settings.tile_size}
          onChange={(e) => setSettings("tile_size", e.target.valueAsNumber)}
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
          class="w-12 bg-transparent border-2"
          value={settings.output_size.x}
          onChange={(e) =>
            setSettings("output_size", "x", e.target.valueAsNumber)
          }
        ></input>
        <span class="mx-1">x</span>
        <input
          type="number"
          class="w-12 bg-transparent border-2"
          value={settings.output_size.y}
          onChange={(e) =>
            setSettings("output_size", "y", e.target.valueAsNumber)
          }
        ></input>
      </div>
      <div class="border-b border-2 border-white my-4 lg:w-full"></div>
      <button class="bg-blue-500 block rounded-md" onClick={loadWfc}>
        Apply
      </button>
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
  const [wfc, setWfc] = createSignal<Wfc | undefined>();
  const [controller, setController] = createSignal<WfcController | undefined>();
  const [loading, setLoading] = createSignal<boolean>(true);
  const [reloadFunc, setReloadFunc] = createSignal<ReloadFunc>(async () => {});

  render(
    () => <PlayControls controller={controller} reload={reloadFunc} />,
    document.getElementById("player-control-bar")!
  );
  render(
    () => (
      <PlayerSettingsMenu
        wfc={wfc}
        controller={controller}
        loading={loading}
        setReloadFunc={setReloadFunc}
      />
    ),
    document.getElementById("player-settings-menu")!
  );

  window.addEventListener("resize", (_e) => {
    if (!controller()) return;

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
    controller()!.resize_canvas(w, h);
  });

  // separated for type checking
  // TODO: handle error case with reject()
  const wfcPromise: Promise<Wfc> = new Promise((resolve) => {
    // Wait for wasm module to load
    const intervalId = setInterval(() => {
      console.log("waiting for wasm init");
      if (window.wasm !== undefined) {
        clearInterval(intervalId);
        resolve(window.wasm as Wfc);
      }
    }, 100);
  });
  // TODO: put this in a createResource()
  wfcPromise
    .then((_wfc) => {
      console.log("wasm loaded");
      setWfc(_wfc);
      return _wfc;
    })
    .then(async (_wfc) => {
      const display = await _wfc.WfcWindow.new();
      const controller = _wfc.WfcController.init(display);
      console.log("attaching window");
      setController(controller);
      setLoading(false);
      return display;
    })
    .then((display) => {
      console.log("starting event loop");
      // NOTE: this is blocking! nothing after this will run!
      display.start_event_loop();
    });
}

init();
