project(
  'learn-opengl',
  'c',
  meson_version : '>= 1.3.0',
  version : '0.1',
  default_options : ['warning_level=3'],
)

dependencies = [
]

sources = [
  'learn_opengl.c',
]

exe = executable(
  'learn-opengl',
  sources,
  dependencies : dependencies,
  install : true,
)

test('basic', exe)
