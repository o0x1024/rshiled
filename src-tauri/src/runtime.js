((globalThis) => {
	const core = Deno.core;

	function argsToMessage(...args) {
		return args.map((arg) => JSON.stringify(arg)).join(" ");
	}

	globalThis.console = {
		log: (...args) => {
			core.print(`[out]: ${argsToMessage(...args)}\n`, false);
		},
		error: (...args) => {
			core.print(`[err]: ${argsToMessage(...args)}\n`, true);
		},
	};

	globalThis.plugin = {
		readFile: (path) => {
			return core.ops.op_read_file(path);
		},
		writeFile: (path, contents) => {
			return core.ops.op_write_file(path, contents);
		},
		removeFile: (path) => {
			return core.ops.op_remove_file(path);
		},
		fetch_get: (url,proxy = '', headers = []) => {
			return core.ops.op_fetch_get(url,proxy,headers);
		},
		fetch_post: (url,data,proxy = '', headers = []) => {
			return core.ops.op_fetch_post(url,data,proxy,headers);
		},
		push_msg: (msg) => {
			return core.ops.op_push_msg(msg);
		},
	};

})(globalThis);
