#if !defined(_MSC_VER) && defined (__STDC_VERSION__) && (__STDC_VERSION__ >= 201112L)
    #include <stdalign.h>
    #define MA_ATOMIC(alignment, type)            _Alignas(alignment) type
#else
    #if defined(__GNUC__)
        /* GCC-style compilers. */
        #define MA_ATOMIC(alignment, type)        type __attribute__((aligned(alignment)))
    #elif defined(_MSC_VER) && _MSC_VER > 1200  /* 1200 = VC6. Alignment not supported, but not necessary because x86 is the only supported target. */
        /* MSVC. */
        #define MA_ATOMIC(alignment, type)        __declspec(align(alignment)) type
    #else
        /* Other compilers. */
        #define MA_ATOMIC(alignment, type)        type
    #endif
#endif


#define MA_ATOMIC_SAFE_TYPE_DECL(c89TypeExtension, typeSize, type) \
    typedef struct \
    { \
        MA_ATOMIC(typeSize, ma_##type) value; \
    } ma_atomic_##type; \

MA_ATOMIC_SAFE_TYPE_DECL(i32, 4, device_state)

