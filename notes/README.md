# Notes and Experiments




## To generate abbreviated header

Create a `dummy.h` file with the following contents:

```c
#define MA_NO_RUNTIME_LINKING
#define MA_ENABLE_ONLY_SPECIFIC_BACKENDS
#define MA_ENABLE_COREAUDIO
#define MINIAUDIO_IMPLEMENTATION
#include "miniaudio.h"
```

Then 

```bash
gcc -E dummy.h > abrev.c

```
