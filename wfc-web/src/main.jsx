import { render } from "solid-js/web";
import { createSignal, createEffect, Show, onMount } from "solid-js";

function PlayControls(props) {
  // NOTE: playing implies pause icon and vice versa
  const [playing, setPlaying] = createSignal(false);

  const toggle = () => {
    props.controller.toggle_playing();
    setPlaying(!playing());
  };

  const PauseIcon = (
    <img
      id="PAUSE_ICON"
      src="https://img.icons8.com/ios-filled/50/FFFFFF/pause--v1.png"
    />
  );

  const PlayIcon = (
    <img
      id="PLAY_ICON"
      src="https://img.icons8.com/ios-filled/50/FFFFFF/play--v1.png"
    />
  );

  const playing_bg = "bg-green-500";
  const pause_bg = "bg-red-500";

  return (
    <>
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
    </>
  );
}

async function wangTileBytes(wang_tile) {
  const path = `./assets/wang/${wang_tile}.png`;
  const bytes = await fetch(path)
    .then((response) => response.blob())
    .then((blob) => {
      const reader = new FileReader();
      reader.readAsArrayBuffer(blob);
      return new Promise((resolve, reject) => {
        reader.onloadend = function () {
          const u8Array = new Uint8Array(reader.result);
          resolve(u8Array);
        };
        reader.onerror = function () {
          reject(reader.error);
        };
      });
    });
  return bytes;
}

async function loadWfc(game, wasm, controller) {
  console.log("Selected preset:", game);
  // assert(window.wasm !== undefined, "wasm not loaded");
  // if (playing) togglePlaying();
  const bytes = await wangTileBytes(game);
  let wfc = wasm.WfcWebBuilder.new_from_image_bytes(bytes)
    .with_tile_size(32)
    .with_output_dimensions(256, 256)
    .wang()
    .build();
  controller.load_wfc(wfc);
}

function ConfigMenu(props) {
  const { wasm, controller } = props;
  const [preset, setPresetSignal] = createSignal("dual");

  function setPreset(value) {
    setPresetSignal(value);
    loadWfc(preset(), wasm, controller);
  }
  onMount(() => {
    setPreset(preset());
  });
  return (
    <>
      <label for="presets">Presets</label>
      <select
        id="presets"
        on:change={(e) => setPreset(e.target.value)}
        value={preset()}
      >
        <option value="dual">Dual</option>
        <option value="celtic">Celtic</option>
      </select>
    </>
  );
}

async function init() {
  console.log("init");
    // Wait for wasm module to load and be added to window
  await new Promise((resolve) => {
    if (window.wasm !== undefined) {
      resolve();
    } else {
      console.log("waiting for wasm");
      const intervalId = setInterval(() => {
        console.log("checking for wasm");
        if (window.wasm !== undefined) {
          clearInterval(intervalId);
          resolve();
        }
      }, 100);
    }
  });
  console.log("wasm loaded");
  let display = await window.wasm.WfcWindow.new();
  const controller = window.wasm.WfcController.init(display);

  render(
    () => <PlayControls controller={controller} />,
    document.getElementById("play-controls")
  );
  render(
    () => <ConfigMenu wasm={window.wasm} controller={controller} />,
    document.getElementById("config-menu")
  );

  // NOTE: this is blocking! nothing after this will run!
  display.start_event_loop();
}

init();
