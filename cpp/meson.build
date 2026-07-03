project(
  'learn-opengl',
  ['c', 'cpp'],
  meson_version : '>= 1.3.0',
  version : '0.1',
  default_options : [
    'warning_level=3',
    'cpp_std=c++17',
  ],
)

cc = meson.get_compiler('c')

glfw_dep = dependency('glfw3', required : true)
opengl_dep = dependency('opengl', required : true)
dl_dep = cc.find_library('dl', required : false)
platform_deps = []

if host_machine.system() == 'darwin'
  platform_deps += dependency('appleframeworks', modules : 'CoreFoundation')
endif

redbook_inc = include_directories('include')
common_deps = [
  glfw_dep,
  opengl_dep,
  dl_dep,
  platform_deps,
]

redbook_common_sources = files(
  'lib/gl3w.c',
  'lib/LoadShaders.cpp',
)

executable(
  '01-triangles',
  files('src/01-triangles/01-triangles.cpp'),
  redbook_common_sources,
  include_directories : redbook_inc,
  dependencies : common_deps,
  install : false,
)
