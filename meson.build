project(
  'wombat',
  'c',
  version: '0.1.0',
  meson_version: '>=1.3.0',
  default_options: [
    'warning_level=2',
  ],
)

bindir = get_option('bindir')
datadir = get_option('datadir')

install_data(
  'data/io.github.archisman_panigrahi.wombat.desktop',
  install_dir: join_paths(datadir, 'applications'),
)

install_data(
  'data/io.github.archisman_panigrahi.wombat.metainfo.xml',
  install_dir: join_paths(datadir, 'metainfo'),
)

install_data(
  'data/io.github.archisman_panigrahi.wombat.png',
  install_dir: join_paths(datadir, 'icons', 'hicolor', '512x512', 'apps'),
)

custom_target(
  'wombat-binary',
  build_by_default: true,
  build_always_stale: true,
  output: 'wombat',
  command: [
    files('build-aux/build-wombat.sh'),
    meson.project_source_root(),
    meson.project_build_root(),
    '@OUTPUT@',
  ],
  env: {
    'CARGO_TARGET_DIR': join_paths(meson.project_build_root(), 'cargo-target'),
  },
  install: true,
  install_dir: bindir,
)