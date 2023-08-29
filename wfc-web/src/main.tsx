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
import { SetStoreFunction, createStore, produce, unwrap } from "solid-js/store";
import type { WfcController, WfcData, PlayerSettings } from "./wfc-web.d.ts";

import type * as WfcNamespace from "./wfc-web.d.ts";
type Wfc = typeof WfcNamespace;

declare global {
    interface Window {
        // script? in Trunk.toml initializes wasm and adds binding
        // to window. Wasm initialization takes a while though so it will
        // be undefined until the promise resolves
        wasm: Wfc | undefined;
    }
}

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

const PlayPauseButton: FC = () => {
    const [ctx, { setPlaying }] = usePlayerContext();
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
                ctx.playing ? "bg-red-500" : "bg-green-500" // red if playing for pause button and green if paused for play button
            } block rounded-md px-4 py-2`}
            onClick={() => setPlaying(!ctx.playing)}
            id="play-pause"
        >
            <Show when={ctx.playing} fallback={PlayIcon}>
                {PauseIcon}
            </Show>
        </button>
    );
};

function PlayControls() {
    const [, { setPlaying, reload: _reload }] = usePlayerContext();

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
                <PlayPauseButton />
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

const PresetSelector: FC<{setPlayerSettings: (s: PlayerSettings) => void}> = (props) => {
    const [{ wfc }, { loadWfc }] = usePlayerContext();
    const [preset, setPreset] = createSignal<PresetImage | null>(null);
    createEffect(async () => {
        let p = preset();
        if (p) {
            const settings = PRESET_SETTINGS[p];
            // apply settings to player version of settings
            props.setPlayerSettings(settings);

            console.log("Selected preset:", p);
            // TODO: put image loading in a createResource()
            let image = await loadPresetImage(p);
            loadWfc(image, settings);
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
                class="border-2 text-white bg-gray-500 rounded-sm p-1"
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
    return (
        <>
    <div class="lg:hidden w-px border-l-2 border-white"></div>
    <div class="hidden lg:block h-px border-b-2 border-white w-full my-3"></div>
        </>
    );
};

const PlayerSettingsSection: FC<{ children: JSX.Element; title?: string }> = (
    props
) => {
    return (
        <div class="flex flex-col text-2xl lg:text-base">
            {props.title && (
                <span>
                    <b>{props.title}</b>
                </span>
            )}
            {props.children}
        </div>
    );
};

const PlayerSettingsMenu: FC = () => {
    let [ctx, { loadWfc }] = usePlayerContext();
    // copy of settings so reset button can reset to original settings
    // not sure if the unwrap is necessary
    let [settings, setSettings] = createStore(ctx.settings);

    const applyChanges = async () => {
        let bytes = await loadPresetImage(settings.image);
        loadWfc(bytes, settings);
    };

    // TODO: resize text in input field text boxes on overflow
    // TODO: make `input` component that does ^ and other repeated logic below
    return (
        <div
            id="config-menu"
            class="flex flex-row lg:flex-col justify-around rounded-md border-2 text-white p-2"
        >
            <PlayerSettingsSection>
                <PresetSelector setPlayerSettings={setSettings}/>
            </PlayerSettingsSection>
            <Divider />
            <PlayerSettingsSection title="Preprocessor Settings">
                <div>
                    <span class="mr-1">Tile Size:</span>
                    <input
                        type="number"
                        class="w-10 bg-transparent border-[1px] truncate hover:whitespace-normal"
                        value={settings.tile_size}
                        onChange={(e) =>
                            setSettings("tile_size", e.target.valueAsNumber)
                        }
                    ></input>
                </div>
            </PlayerSettingsSection>
            <Divider />
            <PlayerSettingsSection title="Model Settings">
                <div>
                    <span class="mr-1">Output Size:</span>
                    <input
                        type="number"
                        class="w-12 bg-transparent border-[1px]"
                        value={settings.output_dimensions.x}
                        onChange={(e) =>
                            setSettings(
                                "output_dimensions",
                                "x",
                                e.target.valueAsNumber
                            )
                        }
                    ></input>
                    <span class="mx-1">x</span>
                    <input
                        type="number"
                        class="w-12 bg-transparent border-[1px]"
                        value={settings.output_dimensions.y}
                        onChange={(e) =>
                            setSettings(
                                "output_dimensions",
                                "y",
                                e.target.valueAsNumber
                            )
                        }
                    ></input>
                </div>
            </PlayerSettingsSection>
            <Divider />
            <button class="bg-blue-500 rounded-md" onClick={applyChanges}>
                <span class="p-2 text-xl">Apply</span>
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
    wfc: WfcInterface;
    playing: boolean;
    settings: PlayerSettings;
    image: Uint8Array;
}

interface PlayerContextApi {
    reload: () => Promise<void>;
    loadWfc: (arg0: Uint8Array, arg1: PlayerSettings) => void;
    setPlaying: (arg0: boolean) => void;
    setState: SetStoreFunction<PlayerContextState>;
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

async function init() {
    type WasmInterface = { loading: true } | { loading: false; wfc: Wfc };
    const [wasm, setWasm] = createSignal<WasmInterface>({ loading: true });
    let default_image = await loadPresetImage(DEFAULT_PRESET);
    const [state, setState] = createStore<PlayerContextState>({
        wfc: { loading: true },
        settings: PRESET_SETTINGS[DEFAULT_PRESET],
        image: default_image,
        playing: false,
    });
    const context: PlayerContextTuple = [
        state,
        {
            async reload(this) {
                if (state.wfc.loading) return;
                console.log("loading:", state.settings.image);
                const bytes = await loadPresetImage(state.settings.image);
                context[1].loadWfc(bytes, state.settings);
            },
            loadWfc(bytes: Uint8Array, settings: PlayerSettings) {
                setState(
                    produce((s) => {
                        s.settings = settings;
                        s.image = bytes;
                    })
                );
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
                state.wfc.controller.set_done_callback(() => {
                    setState("playing", false);
                });
            },
            setPlaying(playing: boolean) {
                console.log("setPlaying", playing);
                setState("playing", (was_playing) => {
                    if (!state.wfc.loading && was_playing !== playing)
                        state.wfc.controller.toggle_playing();
                    return playing;
                });
            },
            setState,
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
            setWasm({ loading: false, wfc: wasm_ });
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
