"use strict";

const chromiumExec = {
  async exec({ input = new ArrayBuffer(0), executable, argv }){
    if(typeof input === "string"){
      input = new TextEncoder().encode(input).buffer;
    }else if(input instanceof Blob){
      input = await new Response(input).arrayBuffer();
    }else if(input instanceof ArrayBuffer){
      // OK
    }else{
      throw Error("Bad \"input\"");
    }

    if(!Array.isArray(argv) || argv.length === 0){
      throw Error("Bad \"argv\"");
    }

    if(typeof executable === "string"){
      // OK
    }else if(executable === undefined){
      executable = argv[0];
    }else{
      throw Error("Bad \"executable\"");
    }

    let result = await new Promise((resolve, reject) => {
      const port = chrome.runtime.connectNative("chromium_exec");

      let stdout = [];
      let stderr = [];

      port.onMessage.addListener(msg => {
        if(msg.type === "stdout"){
          stdout.push(new Uint8Array(msg.data).buffer);
        }else if(msg.type === "stderr"){
          stderr.push(new Uint8Array(msg.data).buffer);
        }else if(msg.type === "terminated"){
          delete msg.type;
          msg.stdout = new Blob(stdout);
          msg.stderr = new Blob(stderr);
          resolve(msg);
        }else if(msg.type === "error"){
          reject(Error(msg.message));
        }else{
          reject(Error("Unknown \"type\""));
        }
      });

      port.onDisconnect.addListener(() => {
        if(chrome.runtime.lastError){
          reject(Error(chrome.runtime.lastError.message));
        }
        reject(Error("Unexpected disconnect"));
      });

      port.postMessage({ request: [[...new Uint8Array(input)], executable, argv] });
    });

    result.stdout = await new Response(result.stdout).arrayBuffer();
    result.stderr = await new Response(result.stderr).arrayBuffer();

    return result;
  },

  async execSuccess(args){
    let result = await chromiumExec.exec(args);

    if(result.reason !== "exited"){
      throw Error("Terminated, but not exited");
    }

    if(result.code !== 0){
      throw Error("Code is not 0");
    }

    delete result.reason;
    delete result.code;

    return result;
  },

  bufferToStr(buffer){
    return new TextDecoder().decode(new Uint8Array(buffer));
  },

  async test(){
    if(chromiumExec.bufferToStr((await chromiumExec.execSuccess({ argv: ["echo", "ok"] })).stdout) !== "ok"){
      throw Error("Failed");
    }
  }
};
