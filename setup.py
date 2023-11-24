import os
import platform
from distutils.core import setup
from distutils.extension import Extension

from Cython.Build import cythonize

if platform.system() == "Darwin":
    os.environ['LDFLAGS'] = " ".join([
            "-framework CoreFoundation",
            # "-framework AudioUnit",
            "-framework AudioToolbox",
            "-framework CoreAudio",
    ])



extensions = [
    Extension("minima", 
        sources=["minima.pyx"],
        define_macros = [],
        include_dirs=[
            "miniaudio",
        ],
        libraries = [
            'm',
            'dl',
            'pthread',
        ],
        library_dirs=[],
        extra_objects=["miniaudio/libminiaudio.a"],
        # extra_compile_args = ["-isystem miniaudio"],
    ),
]


setup(
    name="minima",
    ext_modules=cythonize(extensions, 
        compiler_directives={
            'language_level' : '3',
            'embedsignature': True,
            # 'cdivision': True,      # use C division instead of Python
            # 'boundscheck': True,    # check arrays boundaries
            # 'wraparound': False,    # allow negative indexes to fetch the end of an array

        }),
)
