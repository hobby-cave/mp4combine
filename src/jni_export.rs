use std::ffi::c_void;

use jni::{
    objects::{JClass, JString},
    sys::{jboolean, jint, JNI_ERR, JNI_FALSE, JNI_TRUE, JNI_VERSION_1_4},
    JNIEnv, JavaVM, NativeMethod,
};

use crate::combine::combine_mp4;

macro_rules! jni_call {
    ($e:expr, $desc:literal, $ret:expr) => {
        match $e {
            Ok(v) => v,
            Err(err) => {
                eprintln!("{} error: {}", $desc, err);
                return $ret;
            }
        }
    };
}

#[export_name = "JNI_OnLoad"]
pub extern "C" fn jni_on_load(vm: JavaVM, _reserved: *const c_void) -> jint {
    let env = jni_call!(vm.get_env(), "get JNIEnv", JNI_ERR);
    let class = jni_call!(
        env.find_class("mp4combine/Bridge"),
        "find bridge class",
        JNI_ERR
    );
    jni_call!(
        env.register_native_methods(
            class,
            &[NativeMethod {
                name: "combine".into(),
                sig: "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Z".into(), // bool (init, part, output)
                fn_ptr: jni_combine as *mut c_void,
            }]
        ),
        "register native methods",
        JNI_ERR
    );

    return JNI_VERSION_1_4;
}

extern "C" fn jni_combine(
    env: JNIEnv,
    _type: JClass,
    init: JString,
    part: JString,
    output: JString,
) -> jboolean {
    let init = jni_call!(env.get_string(init), "get init jstr", JNI_FALSE);
    let part = jni_call!(env.get_string(part), "get part jstr", JNI_FALSE);
    let output = jni_call!(env.get_string(output), "get output jstr", JNI_FALSE);

    let init = jni_call!(init.to_str(), "decode init str", JNI_FALSE);
    let part = jni_call!(part.to_str(), "decode part str", JNI_FALSE);
    let output = jni_call!(output.to_str(), "decode output str", JNI_FALSE);

    match combine_mp4(init, part, output) {
        Ok(_) => JNI_TRUE,
        Err(err) => {
            eprintln!("combine error: {}", err);
            JNI_FALSE
        }
    }
}
