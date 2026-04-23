export function main() {
    wasm.main();
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_boolean_get_6ea149f0a8dcc5ff: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_ab4b34d23d6778bd: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_function_3baa9db1a987f47d: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_52ff4ec04186736f: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_undefined_29a43b4d42920abd: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_number_get_c7f42aed0525c451: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_7ed5322991caaec5: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_6b64449b9b9ed33c: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_b46c9b5a9f08ec37: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_activeElement_737cd2e5ce862ac0: function(arg0) {
            const ret = arg0.activeElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_addEventListener_8176dab41b09531c: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_add_0cfb2ab24caa9888: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.add(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_add_2793bb8842a4a861: function() { return handleError(function (arg0, arg1) {
            arg0.add(arg1);
        }, arguments); },
        __wbg_altKey_c4f26b40f1b826b4: function(arg0) {
            const ret = arg0.altKey;
            return ret;
        },
        __wbg_appendChild_e95c8b3b936d250a: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments); },
        __wbg_apply_4c35bd236dda9c14: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.apply(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_arc_817de096f286078c: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.arc(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbg_back_1f680e5e87fac42c: function() { return handleError(function (arg0) {
            arg0.back();
        }, arguments); },
        __wbg_beginPath_6d95cc267dd3e88f: function(arg0) {
            arg0.beginPath();
        },
        __wbg_body_c7b35a55457167ba: function(arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_button_c794bf4b1dcd7c4c: function(arg0) {
            const ret = arg0.button;
            return ret;
        },
        __wbg_cells_a6ba8549c13b79f3: function(arg0) {
            const ret = arg0.cells;
            return ret;
        },
        __wbg_checked_8da9090209958741: function(arg0) {
            const ret = arg0.checked;
            return ret;
        },
        __wbg_childElementCount_eceb0c73c820b066: function(arg0) {
            const ret = arg0.childElementCount;
            return ret;
        },
        __wbg_childNodes_23aa77ec529bb827: function(arg0) {
            const ret = arg0.childNodes;
            return ret;
        },
        __wbg_children_9c299b52b2889289: function(arg0) {
            const ret = arg0.children;
            return ret;
        },
        __wbg_classList_a4e8d7553b666e6d: function(arg0) {
            const ret = arg0.classList;
            return ret;
        },
        __wbg_className_7f4cc742dc43ae94: function(arg0, arg1) {
            const ret = arg1.className;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_clearInterval_d04d8e0ff92c4c05: function(arg0, arg1) {
            arg0.clearInterval(arg1);
        },
        __wbg_clearRect_5fb1d6b44e6b6738: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.clearRect(arg1, arg2, arg3, arg4);
        },
        __wbg_clear_399cfd0dd432663c: function() { return handleError(function (arg0) {
            arg0.clear();
        }, arguments); },
        __wbg_click_bc40376705b1e04d: function(arg0) {
            arg0.click();
        },
        __wbg_clientX_48ead8c93aa7a872: function(arg0) {
            const ret = arg0.clientX;
            return ret;
        },
        __wbg_clientY_ddcce7762c925e13: function(arg0) {
            const ret = arg0.clientY;
            return ret;
        },
        __wbg_closePath_d9cf40637e9c89c2: function(arg0) {
            arg0.closePath();
        },
        __wbg_close_88106990eea7f544: function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments); },
        __wbg_contains_29d51ab38cfd6454: function(arg0, arg1, arg2) {
            const ret = arg0.contains(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_createElement_bbd4c90086fe6f7b: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_createObjectURL_46e1b0c55389893b: function() { return handleError(function (arg0, arg1) {
            const ret = URL.createObjectURL(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_ctrlKey_31968cccd46bdef6: function(arg0) {
            const ret = arg0.ctrlKey;
            return ret;
        },
        __wbg_currentTime_ed6ce93e843d1f92: function(arg0) {
            const ret = arg0.currentTime;
            return ret;
        },
        __wbg_data_bb9dffdd1e99cf2d: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_deleteProperty_d5f7bd763acbdb44: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.deleteProperty(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_deleteRow_f9c35dcc690af106: function() { return handleError(function (arg0, arg1) {
            arg0.deleteRow(arg1);
        }, arguments); },
        __wbg_disabled_0079a9b744e1b8e0: function(arg0) {
            const ret = arg0.disabled;
            return ret;
        },
        __wbg_disabled_3fff0f4da515bc77: function(arg0) {
            const ret = arg0.disabled;
            return ret;
        },
        __wbg_document_7a41071f2f439323: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_duration_dff62afc73b8479d: function(arg0) {
            const ret = arg0.duration;
            return ret;
        },
        __wbg_error_2001591ad2463697: function(arg0) {
            console.error(arg0);
        },
        __wbg_eval_0f5002e126d86aff: function() { return handleError(function (arg0, arg1) {
            const ret = eval(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg_fetch_5ccc4e4f205384ba: function(arg0, arg1, arg2) {
            const ret = arg0.fetch(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_files_7d850950dc306cfe: function(arg0) {
            const ret = arg0.files;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_fillRect_992c5a4646ea7a7f: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.fillRect(arg1, arg2, arg3, arg4);
        },
        __wbg_fillText_dabb33ea287042e2: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.fillText(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments); },
        __wbg_fill_ec5da5f3916cf924: function(arg0) {
            arg0.fill();
        },
        __wbg_focus_089295847acbfa20: function() { return handleError(function (arg0) {
            arg0.focus();
        }, arguments); },
        __wbg_forward_10cd50dc55e1347c: function() { return handleError(function (arg0) {
            arg0.forward();
        }, arguments); },
        __wbg_getAttribute_8627dea35cdb7b06: function(arg0, arg1, arg2, arg3) {
            const ret = arg1.getAttribute(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getContext_fc146f8ec021d074: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getElementById_0b5a508c91194690: function(arg0, arg1, arg2) {
            const ret = arg0.getElementById(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getItem_7fe1351b9ea3b2f3: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getItem(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_getPropertyValue_0bc8c6164d246228: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getPropertyValue(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_get_6011fa3a58f61074: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_8360291721e2339f: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_has_880f1d472f7cecba: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.has(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_height_528848d067cc2221: function(arg0) {
            const ret = arg0.height;
            return ret;
        },
        __wbg_history_22a99931b27987cc: function() { return handleError(function (arg0) {
            const ret = arg0.history;
            return ret;
        }, arguments); },
        __wbg_id_8b383c92c097419c: function(arg0, arg1) {
            const ret = arg1.id;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_innerHTML_df4703c8599a85c4: function(arg0, arg1) {
            const ret = arg1.innerHTML;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_innerText_d5f6dba9855f00c8: function(arg0, arg1) {
            const ret = arg1.innerText;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_instanceof_CanvasRenderingContext2d_24a3fe06e62b98d7: function(arg0) {
            let result;
            try {
                result = arg0 instanceof CanvasRenderingContext2D;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Element_56c8d987654f359e: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Element;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlAnchorElement_6231b5a29726bb69: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLAnchorElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlAudioElement_c2affb8eafa47d12: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLAudioElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlButtonElement_56ddb0781cfac72e: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLButtonElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlCanvasElement_ea4dfc3bb77c734b: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLCanvasElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlElement_47620edd062da622: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlIFrameElement_94ec5e4d6265e8ba: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLIFrameElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlImageElement_f2d6edc5e2cdb758: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLImageElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlInputElement_8dc30e795ec4f2a5: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLInputElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlOptionElement_55d514ca2898322c: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLOptionElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlProgressElement_3df712b456538e81: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLProgressElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlSelectElement_90f5cf3d394847b1: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLSelectElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlTableElement_03607994080b40f5: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLTableElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlTableRowElement_bf98396dc7242e56: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLTableRowElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlTextAreaElement_73ab48e7517a68c2: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLTextAreaElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlVideoElement_a38faa28b470ae86: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLVideoElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Object_7c99480a1cdfb911: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Object;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Response_9b2d111407865ff2: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Response;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_cc64c86c8ef9e02b: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_c3109d14ffc06469: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_item_2756ef1395ab5dda: function(arg0, arg1) {
            const ret = arg0.item(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_item_54db00ffcda8dcfb: function(arg0, arg1) {
            const ret = arg0.item(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_item_aa5667bdc8374f43: function(arg0, arg1) {
            const ret = arg0.item(arg1 >>> 0);
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_keyCode_972708a3ac86591a: function(arg0) {
            const ret = arg0.keyCode;
            return ret;
        },
        __wbg_key_8413ee53931c540f: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg1.key(arg2 >>> 0);
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_keys_2fd1bfdda7e278ca: function(arg0) {
            const ret = Object.keys(arg0);
            return ret;
        },
        __wbg_length_1d7129addb9154f9: function() { return handleError(function (arg0) {
            const ret = arg0.length;
            return ret;
        }, arguments); },
        __wbg_length_3d4ecd04bd8d22f1: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_7d16035ec7aa9d47: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_9107c083c5526b42: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_c157a50ce35f2f72: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_lineTo_c9f1e0dd4824ae31: function(arg0, arg1, arg2) {
            arg0.lineTo(arg1, arg2);
        },
        __wbg_localStorage_f5f66b1ffd2486bc: function() { return handleError(function (arg0) {
            const ret = arg0.localStorage;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_location_73c89ca5bb53ddf3: function(arg0) {
            const ret = arg0.location;
            return ret;
        },
        __wbg_log_7e1aa9064a1dbdbd: function(arg0) {
            console.log(arg0);
        },
        __wbg_max_3550ff7b0788051c: function(arg0, arg1) {
            const ret = arg1.max;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_max_43cfa195eda6d84a: function(arg0) {
            const ret = arg0.max;
            return ret;
        },
        __wbg_min_f693a1a2bfe95ccf: function(arg0, arg1) {
            const ret = arg1.min;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_moveTo_d3deaceb55dc2d80: function(arg0, arg1, arg2) {
            arg0.moveTo(arg1, arg2);
        },
        __wbg_name_9edc86a6da7afd7a: function(arg0, arg1) {
            const ret = arg1.name;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_new_0a8d011ad814b95a: function() { return handleError(function () {
            const ret = new FileReader();
            return ret;
        }, arguments); },
        __wbg_new_2a6e9133304ae2bf: function() { return handleError(function (arg0, arg1) {
            const ret = new WebSocket(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg_new_682678e2f47e32bc: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_aa8d0fa9762c29bd: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_with_str_sequence_and_options_2cfc7ae8f9435aa4: function() { return handleError(function (arg0, arg1) {
            const ret = new Blob(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_offsetHeight_1e906c4f333e7e62: function(arg0) {
            const ret = arg0.offsetHeight;
            return ret;
        },
        __wbg_offsetLeft_bfee0312e0f9bde5: function(arg0) {
            const ret = arg0.offsetLeft;
            return ret;
        },
        __wbg_offsetTop_551e185d17207caa: function(arg0) {
            const ret = arg0.offsetTop;
            return ret;
        },
        __wbg_offsetWidth_c28e4e947f89201d: function(arg0) {
            const ret = arg0.offsetWidth;
            return ret;
        },
        __wbg_offsetX_a88ab66c480b77a3: function(arg0) {
            const ret = arg0.offsetX;
            return ret;
        },
        __wbg_offsetY_679a899b0b60c036: function(arg0) {
            const ret = arg0.offsetY;
            return ret;
        },
        __wbg_options_9427748e81106d12: function(arg0) {
            const ret = arg0.options;
            return ret;
        },
        __wbg_parse_1bbc9c053611d0a7: function() { return handleError(function (arg0, arg1) {
            const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg_pause_4d31251d7f5adac1: function() { return handleError(function (arg0) {
            arg0.pause();
        }, arguments); },
        __wbg_paused_2c6bb20bdf8be8f8: function(arg0) {
            const ret = arg0.paused;
            return ret;
        },
        __wbg_play_c4046eb80d8ddda8: function() { return handleError(function (arg0) {
            const ret = arg0.play();
            return ret;
        }, arguments); },
        __wbg_prepend_21806ba6bff9a559: function() { return handleError(function (arg0, arg1) {
            arg0.prepend(arg1);
        }, arguments); },
        __wbg_prompt_cf3f9b676bbd4b9e: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            const ret = arg1.prompt(getStringFromWasm0(arg2, arg3), getStringFromWasm0(arg4, arg5));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_push_471a5b068a5295f6: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_querySelectorAll_e9e3fbd41310476e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelectorAll(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_querySelector_12b6c7cdf26a3483: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelector(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_querySelector_8d395ebd237ebd46: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelector(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_queueMicrotask_5d15a957e6aa920e: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_queueMicrotask_f8819e5ffc402f36: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_random_ce7f6871aed001dd: function() {
            const ret = Math.random();
            return ret;
        },
        __wbg_readAsText_f5a8c3359a4b84ce: function() { return handleError(function (arg0, arg1) {
            arg0.readAsText(arg1);
        }, arguments); },
        __wbg_removeChild_172df530ec85cc8a: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.removeChild(arg1);
            return ret;
        }, arguments); },
        __wbg_removeEventListener_7bdf07404d9b24bd: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.removeEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_removeItem_487c385a3066a8ed: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.removeItem(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_remove_48cb93cf7a6c4260: function(arg0) {
            arg0.remove();
        },
        __wbg_remove_8aa602fc502f0448: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.remove(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_remove_cd706a2db44da829: function(arg0) {
            arg0.remove();
        },
        __wbg_requestFullscreen_f9701e668f0a74cb: function() { return handleError(function (arg0) {
            arg0.requestFullscreen();
        }, arguments); },
        __wbg_resolve_e6c466bc1052f16c: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_restore_f103803ad0dc390b: function(arg0) {
            arg0.restore();
        },
        __wbg_result_cadfbcadd3b04647: function() { return handleError(function (arg0) {
            const ret = arg0.result;
            return ret;
        }, arguments); },
        __wbg_revokeObjectURL_1d23b31dc4ef5f52: function() { return handleError(function (arg0, arg1) {
            URL.revokeObjectURL(getStringFromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_rotate_fb8a7a0e39ad85a6: function() { return handleError(function (arg0, arg1) {
            arg0.rotate(arg1);
        }, arguments); },
        __wbg_rows_78a0e669a771fc2d: function(arg0) {
            const ret = arg0.rows;
            return ret;
        },
        __wbg_save_5b07d6d1028c3e4d: function(arg0) {
            arg0.save();
        },
        __wbg_selectedIndex_35684b759950b517: function(arg0) {
            const ret = arg0.selectedIndex;
            return ret;
        },
        __wbg_selectionEnd_d3121c52b43b7ee4: function() { return handleError(function (arg0) {
            const ret = arg0.selectionEnd;
            return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
        }, arguments); },
        __wbg_selectionStart_0c3341edd4f2adab: function() { return handleError(function (arg0) {
            const ret = arg0.selectionStart;
            return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
        }, arguments); },
        __wbg_send_15358dbe221c6258: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.send(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_setAttribute_6fde4098d274155c: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setInterval_2aed0ef9e1a3b117: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.setInterval(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_setItem_e6399d3faae141dc: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setProperty_0d903d23a71dfe70: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_set_022bee52d0b05b19: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_autoplay_0cb6c503c0553cf6: function(arg0, arg1) {
            arg0.autoplay = arg1 !== 0;
        },
        __wbg_set_checked_e37eb600f8ec831d: function(arg0, arg1) {
            arg0.checked = arg1 !== 0;
        },
        __wbg_set_className_1c8245364f86eb8d: function(arg0, arg1, arg2) {
            arg0.className = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_controls_b9c30a4906019bd1: function(arg0, arg1) {
            arg0.controls = arg1 !== 0;
        },
        __wbg_set_currentTime_8dff14aaffa6af0b: function(arg0, arg1) {
            arg0.currentTime = arg1;
        },
        __wbg_set_disabled_62644deb31d137bb: function(arg0, arg1) {
            arg0.disabled = arg1 !== 0;
        },
        __wbg_set_disabled_8dab3b8eede06d80: function(arg0, arg1) {
            arg0.disabled = arg1 !== 0;
        },
        __wbg_set_disabled_a78da97db89c031f: function(arg0, arg1) {
            arg0.disabled = arg1 !== 0;
        },
        __wbg_set_disabled_db1f99009ea012dc: function(arg0, arg1) {
            arg0.disabled = arg1 !== 0;
        },
        __wbg_set_download_f60111fb9c3eac2d: function(arg0, arg1, arg2) {
            arg0.download = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_fillStyle_e51447e54357dc46: function(arg0, arg1, arg2) {
            arg0.fillStyle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_font_295aa505e45244aa: function(arg0, arg1, arg2) {
            arg0.font = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_globalAlpha_1660b0603161d11b: function(arg0, arg1) {
            arg0.globalAlpha = arg1;
        },
        __wbg_set_hash_0030baed6e93eb7b: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.hash = getStringFromWasm0(arg1, arg2);
        }, arguments); },
        __wbg_set_height_be9b2b920bd68401: function(arg0, arg1) {
            arg0.height = arg1 >>> 0;
        },
        __wbg_set_href_f253ea395272f0c4: function(arg0, arg1, arg2) {
            arg0.href = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_id_e241e8155f198c0e: function(arg0, arg1, arg2) {
            arg0.id = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_innerHTML_a3c82996073b31ea: function(arg0, arg1, arg2) {
            arg0.innerHTML = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_innerText_edb4a0283df8c609: function(arg0, arg1, arg2) {
            arg0.innerText = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_lineWidth_2fae105117e1a89f: function(arg0, arg1) {
            arg0.lineWidth = arg1;
        },
        __wbg_set_loop_3d9e7982bafcbe6b: function(arg0, arg1) {
            arg0.loop = arg1 !== 0;
        },
        __wbg_set_maxLength_e59961d0d4befc12: function(arg0, arg1) {
            arg0.maxLength = arg1;
        },
        __wbg_set_max_0b9aeaecc1dca919: function(arg0, arg1) {
            arg0.max = arg1;
        },
        __wbg_set_max_70a94f27b212a148: function(arg0, arg1, arg2) {
            arg0.max = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_min_43029b186aa2af25: function(arg0, arg1, arg2) {
            arg0.min = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_multiple_34474eb0d8a7467a: function(arg0, arg1) {
            arg0.multiple = arg1 !== 0;
        },
        __wbg_set_onchange_7f5b40e8b1b4340f: function(arg0, arg1) {
            arg0.onchange = arg1;
        },
        __wbg_set_onclose_17fa3bbcc4ba3541: function(arg0, arg1) {
            arg0.onclose = arg1;
        },
        __wbg_set_onerror_da99c4232662a084: function(arg0, arg1) {
            arg0.onerror = arg1;
        },
        __wbg_set_onload_ebc9de0780d038a2: function(arg0, arg1) {
            arg0.onload = arg1;
        },
        __wbg_set_onmessage_c1db358b9c38e3f1: function(arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_onopen_cd47b8fb1d92dee9: function(arg0, arg1) {
            arg0.onopen = arg1;
        },
        __wbg_set_poster_34f8847ca51fed9e: function(arg0, arg1, arg2) {
            arg0.poster = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_readOnly_58c63f33f95e4b99: function(arg0, arg1) {
            arg0.readOnly = arg1 !== 0;
        },
        __wbg_set_readOnly_c59c89d3cd729606: function(arg0, arg1) {
            arg0.readOnly = arg1 !== 0;
        },
        __wbg_set_selectedIndex_aa84f9e9491df9b9: function(arg0, arg1) {
            arg0.selectedIndex = arg1;
        },
        __wbg_set_selectionEnd_4b8448e5dac64777: function() { return handleError(function (arg0, arg1) {
            arg0.selectionEnd = arg1 === 0x100000001 ? undefined : arg1;
        }, arguments); },
        __wbg_set_selectionStart_b92cfc0f30c8c1c4: function() { return handleError(function (arg0, arg1) {
            arg0.selectionStart = arg1 === 0x100000001 ? undefined : arg1;
        }, arguments); },
        __wbg_set_src_131d0c69b284b037: function(arg0, arg1, arg2) {
            arg0.src = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_src_59f1be1c833b4918: function(arg0, arg1, arg2) {
            arg0.src = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_src_e142cf6609ec7e5c: function(arg0, arg1, arg2) {
            arg0.src = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_strokeStyle_0429d48dae657e53: function(arg0, arg1, arg2) {
            arg0.strokeStyle = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_tabIndex_f1f5240b07c77382: function(arg0, arg1) {
            arg0.tabIndex = arg1;
        },
        __wbg_set_textAlign_e1a83482b00339b8: function(arg0, arg1, arg2) {
            arg0.textAlign = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_textContent_223eb7313f8a7178: function(arg0, arg1, arg2) {
            arg0.textContent = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_text_7ad43ddc2503aa08: function(arg0, arg1, arg2) {
            arg0.text = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_title_f3a9c2c43e164d11: function(arg0, arg1, arg2) {
            arg0.title = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_type_15e4214f5c54262e: function(arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_type_8b2743f6b4de4035: function(arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_value_0bcc28b703f90ca9: function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_value_6afdfce42dec7768: function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_value_d3cd5a0ce61506cd: function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_value_d84be184846d017b: function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_value_ec2478b7e949068d: function(arg0, arg1) {
            arg0.value = arg1;
        },
        __wbg_set_volume_1741406a2f0b0ce9: function(arg0, arg1) {
            arg0.volume = arg1;
        },
        __wbg_set_width_5cda41d4d06a14dd: function(arg0, arg1) {
            arg0.width = arg1 >>> 0;
        },
        __wbg_set_wrap_a4609dbe4178500b: function(arg0, arg1, arg2) {
            arg0.wrap = getStringFromWasm0(arg1, arg2);
        },
        __wbg_shiftKey_e483c13c966878f6: function(arg0) {
            const ret = arg0.shiftKey;
            return ret;
        },
        __wbg_static_accessor_GLOBAL_8cfadc87a297ca02: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_602256ae5c8f42cf: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_e445c1c7484aecc3: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_f20e8576ef1e0f17: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_stopPropagation_e088fca8231e68c4: function(arg0) {
            arg0.stopPropagation();
        },
        __wbg_stringify_91082ed7a5a5769e: function() { return handleError(function (arg0) {
            const ret = JSON.stringify(arg0);
            return ret;
        }, arguments); },
        __wbg_stringify_fcac92843dea23ae: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = JSON.stringify(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_strokeRect_502699d92aeb85f1: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.strokeRect(arg1, arg2, arg3, arg4);
        },
        __wbg_stroke_42f07013960cf81b: function(arg0) {
            arg0.stroke();
        },
        __wbg_style_c331a9f6564f8f62: function(arg0) {
            const ret = arg0.style;
            return ret;
        },
        __wbg_tagName_a6d6785a7c70fca2: function(arg0, arg1) {
            const ret = arg1.tagName;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_target_6d97e221d11b71b6: function(arg0) {
            const ret = arg0.target;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_textContent_1f28330a124ec047: function(arg0, arg1) {
            const ret = arg1.textContent;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_text_595ef75535aa25c1: function() { return handleError(function (arg0) {
            const ret = arg0.text();
            return ret;
        }, arguments); },
        __wbg_then_792e0c862b060889: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_8e16ee11f05e4827: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_title_cab85f76b4678341: function(arg0, arg1) {
            const ret = arg1.title;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_toggle_decbabc8f8ea5038: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.toggle(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_translate_34989493d69eaecd: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.translate(arg1, arg2);
        }, arguments); },
        __wbg_value_355e920912cca1c3: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_value_6079dd28568d83c9: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_value_acdc3fed0dd155ba: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_value_bcc6c70014ee4ddf: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_volume_5b42e6078d69c6d6: function(arg0) {
            const ret = arg0.volume;
            return ret;
        },
        __wbg_warn_3cc416af27dbdc02: function(arg0) {
            console.warn(arg0);
        },
        __wbg_width_5adcb07d04d08bdf: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 129, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__hbbd456a9b356e462);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Event")], shim_idx: 100, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("KeyboardEvent")], shim_idx: 100, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_2);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 100, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_3);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("MouseEvent")], shim_idx: 100, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_4);
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("ProgressEvent")], shim_idx: 100, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_5);
            return ret;
        },
        __wbindgen_cast_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 103, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen__convert__closures_____invoke__hdcacbf2424d72ac0);
            return ret;
        },
        __wbindgen_cast_0000000000000008: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000009: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./hang_test_bg.js": import0,
    };
}

function wasm_bindgen__convert__closures_____invoke__hdcacbf2424d72ac0(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__hdcacbf2424d72ac0(arg0, arg1);
}

function wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_2(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_2(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_3(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_3(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_4(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_4(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_5(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h21ac2589d8654ed9_5(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__hbbd456a9b356e462(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__hbbd456a9b356e462(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('hang_test_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
