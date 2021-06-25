import os
from distutils.core import setup
from distutils.extension import Extension

from Cython.Build import cythonize

os.environ['LDFLAGS'] = " ".join([
        # "-framework CoreServices",
        "-framework CoreFoundation",
        "-framework AudioUnit",
        # "-framework AudioToolbox",
        "-framework CoreAudio",
])

extensions = [
    Extension("mini", 
        sources=["mini.pyx", "miniaudio/miniaudio.c"],
        define_macros = [
            # ('MINIAUDIO_IMPLEMENTATION', 1),
        ],
        include_dirs=[
            "miniaudio",
        ],
        libraries = [
            'm',
            'dl',
            'pthread',
        ],
        library_dirs=[],
        extra_objects=[],
        extra_compile_args = ["-isystem miniaudio"],
    ),
]


setup(
    name="miniaudio in cython",
    ext_modules=cythonize(extensions, 
        compiler_directives={
            'language_level' : '3',
            'embedsignature': True,
            # 'cdivision': True,      # use C division instead of Python
            # 'boundscheck': True,    # check arrays boundaries
            # 'wraparound': False,    # allow negative indexes to fetch the end of an array

        }),
)
