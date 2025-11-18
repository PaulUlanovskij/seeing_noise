run: build
	python3 -m http.server
build: src/
	wasm-pack build --target web --out-dir pkg
