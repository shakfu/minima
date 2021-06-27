# Notes


## To generate abbreviated header

This head is used implement the cython pxd file.

Create a `dummy.h` file with the following contents:

```c
#define MA_NO_RUNTIME_LINKING
#define MINIAUDIO_IMPLEMENTATION
#include "miniaudio.h"
```

Then 

```bash
gcc -E dummy.h > abrev.h

```
