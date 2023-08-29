import { render } from "solid-js/web";
import {
    createSignal,
    Show,
    createEffect,
    Accessor,
    Component as FC,
    createContext,
    JSX,
    useContext,
} from "solid-js";
import { createStore, produce } from "solid-js/store";
import type { WfcController, WfcData, PlayerSettings } from "./wfc-web.d.ts";

import type * as WfcNamespace from "./wfc-web.d.ts";
type Wfc = typeof WfcNamespace;

type ReloadFunc = () => Promise<void>;

const DEFAULT_PRESET: PresetImage = "dual";

const PRESET_SETTINGS: Record<string, PlayerSettings> = {
    dual: {
        tile_size: 32,
        output_dimensions: { x: 256, y: 256 },
        wang: true,
        image: "dual",
    },
    celtic: {
        tile_size: 32,
        output_dimensions: { x: 256, y: 256 },
        wang: true,
        image: "celtic",
    },
    corners: {
        tile_size: 3,
        output_dimensions: { x: 60, y: 60 },
        wang: false,
        image: "corners",
    },
};

type PresetImage = keyof typeof PRESET_SETTINGS;

interface PlayPauseButtonProps {
    playing: Accessor<boolean>;
    toggle: () => void;
}
const PlayPauseButton: FC<PlayPauseButtonProps> = ({ playing, toggle }) => {
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
    );
};

function PlayControls() {
    const [{ wfc, reload: _reload }] = usePlayerContext();
    // NOTE: playing implies pause icon and vice versa
    const [playing, setPlaying] = createSignal(false);

    const toggle = () => {
        setPlaying(!playing());
        if (!wfc.loading) wfc.controller.toggle_playing();
    };
    const reload = () => {
        setPlaying(false);
        _reload();
    };

    return (
        <div
            id="play-controls"
            class="z-50 bottom-0 flex flex-row justify-between my-2"
        >
            <div id="left-play-controls">
                <PlayPauseButton playing={playing} toggle={toggle} />
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
    const path = `./assets/${preset.wang ? "wang" : "non-wang"}/${
        preset.image
    }.png`;
    const bytes: Uint8Array = await fetch(path)
        .then((response) => response.blob())
        .then((blob) => {
            const reader = new FileReader();
            reader.readAsArrayBuffer(blob);
            return new Promise((resolve, reject) => {
                reader.onloadend = function () {
                    const u8Array = new Uint8Array(
                        reader.result as ArrayBufferLike
                    );
                    resolve(u8Array);
                };
                reader.onerror = function () {
                    reject(reader.error);
                };
            });
        });
    return bytes;
}

interface PresetSelectorProps {
    setSettings: (arg0: PlayerSettings) => void;
}

const PresetSelector: FC<PresetSelectorProps> = (props) => {
    const [{ wfc }, {startWfc}] = usePlayerContext();
    const [preset, setPreset] = createSignal<PresetImage | null>(null);
    createEffect(async () => {
        let p = preset();
        if (p) {
            const settings = PRESET_SETTINGS[p];
            props.setSettings(settings);

            if (!wfc.loading) {
                console.log("Selected preset:", p);
                // TODO: put image loading in a createResource()
                let image = await loadPresetImage(p);
                startWfc(image, settings)

            } else {
                console.log("wfc not loaded");
            }
        }
    });
    // set preset once wasm is done loading
    createEffect(() => {
        if (!wfc.loading) {
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
};

const Divider: FC = () => {
    return <div class="border-b border-2 border-white my-4 lg:w-full"></div>;
};

const PlayerSettingsMenu: FC = () => {
    let [{ wfc }, { setReload, startWfc }] = usePlayerContext();

    const [settings, setSettings] = createStore<PlayerSettings>(
        PRESET_SETTINGS[DEFAULT_PRESET]
    );
    async function loadWfc() {
        if (!wfc.loading) {
            console.log("loading:", settings.image);
            const bytes = await loadPresetImage(settings.image);
            startWfc(bytes, settings);
        }
    }
    setReload(loadWfc);
    // TODO: resize text in input field text boxes on overflow
    // TODO: make `input` component that does ^ and other repeated logic below
    return (
        <div
            id="config-menu"
            class="flex shrink flex-row lg:flex-col rounded-md border-2 text-white p-2"
        >
            <PresetSelector setSettings={setSettings} />
            <Divider />
            <span>
                <b>Preprocessor Settings</b>
            </span>
            <div class="flex-row">
                <span class="mr-1">Tile Size:</span>
                <input
                    type="number"
                    class="w-10 bg-transparent border-2 truncate hover:whitespace-normal"
                    value={settings.tile_size}
                    onChange={(e) =>
                        setSettings("tile_size", e.target.valueAsNumber)
                    }
                ></input>
            </div>
            <Divider />
            <span>
                <b>Model Settings</b>
            </span>
            <div class="flex-row">
                <span class="mr-1">Output Size:</span>
                <input
                    type="number"
                    class="w-12 bg-transparent border-2"
                    value={settings.output_dimensions.x}
                    onChange={(e) =>
                        setSettings("output_dimensions", "x", e.target.valueAsNumber)
                    }
                ></input>
                <span class="mx-1">x</span>
                <input
                    type="number"
                    class="w-12 bg-transparent border-2"
                    value={settings.output_dimensions.y}
                    onChange={(e) =>
                        setSettings("output_dimensions", "y", e.target.valueAsNumber)
                    }
                ></input>
            </div>
            <Divider />
            <button class="bg-blue-500 block rounded-md" onClick={loadWfc}>
                Apply
            </button>
        </div>
    );
};

const PlayerMenu: FC = () => {
    return (
        <div class="mx-4">
            <PlayerSettingsMenu />
            <PlayControls />
        </div>
    );
};

type WfcInterface =
    | {
          loading: true;
      }
    | {
          loading: false;
          controller: WfcController;
      };

interface PlayerContextState {
    reload: ReloadFunc;
    wfc: WfcInterface;
}

interface PlayerContextApi {
    setReload: (arg0: ReloadFunc) => void;
    startWfc: (arg0: Uint8Array, arg1: PlayerSettings) => void;
}

type PlayerContextTuple = [PlayerContextState, PlayerContextApi];

const PlayerContext = createContext<PlayerContextTuple>();

const usePlayerContext = () => {
    const ctx = useContext(PlayerContext);
    if (!ctx)
        throw new Error(
            "usePlayerContext must be used within a PlayerContextProvider"
        );
    return ctx;
};

declare global {
    interface Window {
        // script? in Trunk.toml initializes wasm and adds binding
        // to window. Wasm initialization takes a while though so it will
        // be undefined until the promise resolves
        wasm: Wfc | undefined;
    }
}

async function init() {
    type WasmInterface = { loading: true } | { loading: false; wfc: Wfc };
    const [wasm, setWasm] = createSignal<WasmInterface>({ loading: true });
    const [state, setState] = createStore<PlayerContextState>({
        wfc: { loading: true },
        reload: async () => {},
    });
    const context: PlayerContextTuple = [
        state,
        {
            setReload(f: ReloadFunc) {
                setState("reload", f);
            },
            startWfc(bytes: Uint8Array, settings: PlayerSettings) {
                let wasm_ = wasm();
                if (wasm_.loading) {
                    return;
                }
                let wfcData = wasm_.wfc.WfcWebBuilder.build_from_json_settings(
                    bytes,
                    settings
                );
                if (state.wfc.loading) {
                    return;
                }
                state.wfc.controller.load_wfc(wfcData);
            },
        },
    ];

    render(
        () => (
            <PlayerContext.Provider value={context}>
                <PlayerMenu />
            </PlayerContext.Provider>
        ),
        document.getElementById("player-menu")!
    );

    window.addEventListener("resize", (_e) => {
        if (state.wfc.loading) return;
        let controller = state.wfc.controller;

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
        .then(async (wasm_) => {
            console.log("wasm loaded");
            const display = await wasm_.WfcWindow.new();
            const controller = wasm_.WfcController.init(display);
            console.log("attaching window");
            setWasm({ loading: false, wfc: wasm_ })
            setState("wfc", { loading: false, controller });
            return display;
        })
        .then((display) => {
            console.log("starting event loop");
            // NOTE: this is blocking! nothing after this will run!
            display.start_event_loop();
        });
}

init();
