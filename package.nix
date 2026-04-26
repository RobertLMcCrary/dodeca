{
  lib,
  stdenv,
  rustPlatform,
  pkg-config,
  cmake,
  buildWasmBindgenCli,
  fetchCrate,
  zlib,
  libiconv,
  libcap ? null,
  xorg ? null,
  apple-sdk ? null,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "dodeca";
  version = "0.6.1";

  src = lib.cleanSource ./.;

  # Single vendored-deps hash (update by building once).
  cargoHash = "sha256-fL4tQ6MJRjLhtIjUkmVUhsKvw5Vj4+zIuW5k8sv9GGY=";

  doCheck = false;

  # wasm-bindgen-cli must match wasm-bindgen (=0.2.108) exactly.
  nativeBuildInputs = [
    pkg-config
    cmake
    (buildWasmBindgenCli {
      src = fetchCrate {
        pname = "wasm-bindgen-cli";
        version = "0.2.108";
        hash = "sha256-UsuxILm1G6PkmVw0I/JF12CRltAfCJQFOaT4hFwvR8E=";
      };
      cargoDeps = rustPlatform.fetchCargoVendor {
        src = fetchCrate {
          pname = "wasm-bindgen-cli";
          version = "0.2.108";
          hash = "sha256-UsuxILm1G6PkmVw0I/JF12CRltAfCJQFOaT4hFwvR8E=";
        };
        pname = "wasm-bindgen-cli";
        version = "0.2.108";
        hash = "sha256-iqQiWbsKlLBiJFeqIYiXo3cqxGLSjNM8SOWXGM9u43E=";
      };
    })
  ];

  buildInputs = [
    zlib
  ]
  ++ lib.optionals stdenv.isDarwin [
    libiconv
    apple-sdk
  ]
  ++ lib.optionals stdenv.isLinux [
    libcap
    xorg.libX11
    xorg.libXcursor
    xorg.libXi
    xorg.libXrandr
  ];

  meta = with lib; {
    description = "A fully incremental static site generator";
    longDescription = ''
      dodeca is a fully incremental static site generator designed for speed and
      correctness. It features a custom template engine (Gingembre), a plugin
      architecture for specialised tasks (images, fonts, CSS, etc.), and a
      development mode that perfectly matches production output.
    '';
    homepage = "https://github.com/bearcove/dodeca";
    license = with licenses; [
      mit
      asl20
    ];
    maintainers = with maintainers; [ ];
    mainProgram = "ddc";
    platforms = platforms.unix;
  };
})
