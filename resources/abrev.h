

typedef enum
{
    MA_LOG_LEVEL_DEBUG = 4,
    MA_LOG_LEVEL_INFO = 3,
    MA_LOG_LEVEL_WARNING = 2,
    MA_LOG_LEVEL_ERROR = 1
} ma_log_level

typedef struct ma_context ma_context
typedef struct ma_device ma_device

typedef ma_uint8 ma_channel

typedef enum
{
    MA_CHANNEL_NONE = 0,
    MA_CHANNEL_MONO = 1,
    MA_CHANNEL_FRONT_LEFT = 2,
    MA_CHANNEL_FRONT_RIGHT = 3,
    MA_CHANNEL_FRONT_CENTER = 4,
    MA_CHANNEL_LFE = 5,
    MA_CHANNEL_BACK_LEFT = 6,
    MA_CHANNEL_BACK_RIGHT = 7,
    MA_CHANNEL_FRONT_LEFT_CENTER = 8,
    MA_CHANNEL_FRONT_RIGHT_CENTER = 9,
    MA_CHANNEL_BACK_CENTER = 10,
    MA_CHANNEL_SIDE_LEFT = 11,
    MA_CHANNEL_SIDE_RIGHT = 12,
    MA_CHANNEL_TOP_CENTER = 13,
    MA_CHANNEL_TOP_FRONT_LEFT = 14,
    MA_CHANNEL_TOP_FRONT_CENTER = 15,
    MA_CHANNEL_TOP_FRONT_RIGHT = 16,
    MA_CHANNEL_TOP_BACK_LEFT = 17,
    MA_CHANNEL_TOP_BACK_CENTER = 18,
    MA_CHANNEL_TOP_BACK_RIGHT = 19,
    MA_CHANNEL_AUX_0 = 20,
    MA_CHANNEL_AUX_1 = 21,
    MA_CHANNEL_AUX_2 = 22,
    MA_CHANNEL_AUX_3 = 23,
    MA_CHANNEL_AUX_4 = 24,
    MA_CHANNEL_AUX_5 = 25,
    MA_CHANNEL_AUX_6 = 26,
    MA_CHANNEL_AUX_7 = 27,
    MA_CHANNEL_AUX_8 = 28,
    MA_CHANNEL_AUX_9 = 29,
    MA_CHANNEL_AUX_10 = 30,
    MA_CHANNEL_AUX_11 = 31,
    MA_CHANNEL_AUX_12 = 32,
    MA_CHANNEL_AUX_13 = 33,
    MA_CHANNEL_AUX_14 = 34,
    MA_CHANNEL_AUX_15 = 35,
    MA_CHANNEL_AUX_16 = 36,
    MA_CHANNEL_AUX_17 = 37,
    MA_CHANNEL_AUX_18 = 38,
    MA_CHANNEL_AUX_19 = 39,
    MA_CHANNEL_AUX_20 = 40,
    MA_CHANNEL_AUX_21 = 41,
    MA_CHANNEL_AUX_22 = 42,
    MA_CHANNEL_AUX_23 = 43,
    MA_CHANNEL_AUX_24 = 44,
    MA_CHANNEL_AUX_25 = 45,
    MA_CHANNEL_AUX_26 = 46,
    MA_CHANNEL_AUX_27 = 47,
    MA_CHANNEL_AUX_28 = 48,
    MA_CHANNEL_AUX_29 = 49,
    MA_CHANNEL_AUX_30 = 50,
    MA_CHANNEL_AUX_31 = 51,
    MA_CHANNEL_LEFT = MA_CHANNEL_FRONT_LEFT,
    MA_CHANNEL_RIGHT = MA_CHANNEL_FRONT_RIGHT,
    MA_CHANNEL_POSITION_COUNT = (MA_CHANNEL_AUX_31 + 1)
} _ma_channel_position

typedef enum
{
    MA_SUCCESS = 0,
    MA_ERROR = -1,
    MA_INVALID_ARGS = -2,
    MA_INVALID_OPERATION = -3,
    MA_OUT_OF_MEMORY = -4,
    MA_OUT_OF_RANGE = -5,
    MA_ACCESS_DENIED = -6,
    MA_DOES_NOT_EXIST = -7,
    MA_ALREADY_EXISTS = -8,
    MA_TOO_MANY_OPEN_FILES = -9,
    MA_INVALID_FILE = -10,
    MA_TOO_BIG = -11,
    MA_PATH_TOO_LONG = -12,
    MA_NAME_TOO_LONG = -13,
    MA_NOT_DIRECTORY = -14,
    MA_IS_DIRECTORY = -15,
    MA_DIRECTORY_NOT_EMPTY = -16,
    MA_AT_END = -17,
    MA_NO_SPACE = -18,
    MA_BUSY = -19,
    MA_IO_ERROR = -20,
    MA_INTERRUPT = -21,
    MA_UNAVAILABLE = -22,
    MA_ALREADY_IN_USE = -23,
    MA_BAD_ADDRESS = -24,
    MA_BAD_SEEK = -25,
    MA_BAD_PIPE = -26,
    MA_DEADLOCK = -27,
    MA_TOO_MANY_LINKS = -28,
    MA_NOT_IMPLEMENTED = -29,
    MA_NO_MESSAGE = -30,
    MA_BAD_MESSAGE = -31,
    MA_NO_DATA_AVAILABLE = -32,
    MA_INVALID_DATA = -33,
    MA_TIMEOUT = -34,
    MA_NO_NETWORK = -35,
    MA_NOT_UNIQUE = -36,
    MA_NOT_SOCKET = -37,
    MA_NO_ADDRESS = -38,
    MA_BAD_PROTOCOL = -39,
    MA_PROTOCOL_UNAVAILABLE = -40,
    MA_PROTOCOL_NOT_SUPPORTED = -41,
    MA_PROTOCOL_FAMILY_NOT_SUPPORTED = -42,
    MA_ADDRESS_FAMILY_NOT_SUPPORTED = -43,
    MA_SOCKET_NOT_SUPPORTED = -44,
    MA_CONNECTION_RESET = -45,
    MA_ALREADY_CONNECTED = -46,
    MA_NOT_CONNECTED = -47,
    MA_CONNECTION_REFUSED = -48,
    MA_NO_HOST = -49,
    MA_IN_PROGRESS = -50,
    MA_CANCELLED = -51,
    MA_MEMORY_ALREADY_MAPPED = -52,
    MA_CRC_MISMATCH = -100,
    MA_FORMAT_NOT_SUPPORTED = -200,
    MA_DEVICE_TYPE_NOT_SUPPORTED = -201,
    MA_SHARE_MODE_NOT_SUPPORTED = -202,
    MA_NO_BACKEND = -203,
    MA_NO_DEVICE = -204,
    MA_API_NOT_FOUND = -205,
    MA_INVALID_DEVICE_CONFIG = -206,
    MA_LOOP = -207,
    MA_BACKEND_NOT_ENABLED = -208,
    MA_DEVICE_NOT_INITIALIZED = -300,
    MA_DEVICE_ALREADY_INITIALIZED = -301,
    MA_DEVICE_NOT_STARTED = -302,
    MA_DEVICE_NOT_STOPPED = -303,
    MA_FAILED_TO_INIT_BACKEND = -400,
    MA_FAILED_TO_OPEN_BACKEND_DEVICE = -401,
    MA_FAILED_TO_START_BACKEND_DEVICE = -402,
    MA_FAILED_TO_STOP_BACKEND_DEVICE = -403
} ma_result

typedef enum
{
    ma_stream_format_pcm = 0
} ma_stream_format

typedef enum
{
    ma_stream_layout_interleaved = 0,
    ma_stream_layout_deinterleaved
} ma_stream_layout

typedef enum
{
    ma_dither_mode_none = 0,
    ma_dither_mode_rectangle,
    ma_dither_mode_triangle
} ma_dither_mode

typedef enum
{
    ma_format_unknown = 0,
    ma_format_u8 = 1,
    ma_format_s16 = 2,
    ma_format_s24 = 3,
    ma_format_s32 = 4,
    ma_format_f32 = 5,
    ma_format_count
} ma_format

typedef enum
{

    ma_standard_sample_rate_48000 = 48000,
    ma_standard_sample_rate_44100 = 44100,

    ma_standard_sample_rate_32000 = 32000,
    ma_standard_sample_rate_24000 = 24000,
    ma_standard_sample_rate_22050 = 22050,

    ma_standard_sample_rate_88200 = 88200,
    ma_standard_sample_rate_96000 = 96000,
    ma_standard_sample_rate_176400 = 176400,
    ma_standard_sample_rate_192000 = 192000,

    ma_standard_sample_rate_16000 = 16000,
    ma_standard_sample_rate_11025 = 11025,
    ma_standard_sample_rate_8000 = 8000,

    ma_standard_sample_rate_352800 = 352800,
    ma_standard_sample_rate_384000 = 384000,

    ma_standard_sample_rate_min = ma_standard_sample_rate_8000,
    ma_standard_sample_rate_max = ma_standard_sample_rate_384000,
    ma_standard_sample_rate_count = 14
} ma_standard_sample_rate


typedef enum
{
    ma_channel_mix_mode_rectangular = 0,
    ma_channel_mix_mode_simple,
    ma_channel_mix_mode_custom_weights,
    ma_channel_mix_mode_default = ma_channel_mix_mode_rectangular
} ma_channel_mix_mode

typedef enum
{
    ma_standard_channel_map_microsoft,
    ma_standard_channel_map_alsa,
    ma_standard_channel_map_rfc3551,
    ma_standard_channel_map_flac,
    ma_standard_channel_map_vorbis,
    ma_standard_channel_map_sound4,
    ma_standard_channel_map_sndio,
    ma_standard_channel_map_webaudio = ma_standard_channel_map_flac,
    ma_standard_channel_map_default = ma_standard_channel_map_microsoft
} ma_standard_channel_map

typedef enum
{
    ma_performance_profile_low_latency = 0,
    ma_performance_profile_conservative
} ma_performance_profile


typedef struct
{
    void* pUserData
    void* (* onMalloc)(size_t sz, void* pUserData)
    void* (* onRealloc)(void* p, size_t sz, void* pUserData)
    void (* onFree)(void* p, void* pUserData)
} ma_allocation_callbacks

typedef struct
{
    ma_int32 state
} ma_lcg

typedef struct { _Alignas(4) ma_uint32 value } ma_atomic_uint32
typedef struct { _Alignas(4) ma_int32 value } ma_atomic_int32
typedef struct { _Alignas(8) ma_uint64 value } ma_atomic_uint64
typedef struct { _Alignas(4) ma_float value } ma_atomic_float
typedef struct { _Alignas(4) ma_bool32 value } ma_atomic_bool32



typedef ma_uint32 ma_spinlock



typedef enum
{
    ma_thread_priority_idle = -5,
    ma_thread_priority_lowest = -4,
    ma_thread_priority_low = -3,
    ma_thread_priority_normal = -2,
    ma_thread_priority_high = -1,
    ma_thread_priority_highest = 0,
    ma_thread_priority_realtime = 1,
    ma_thread_priority_default = 0
} ma_thread_priority


typedef ma_pthread_t ma_thread





typedef ma_pthread_mutex_t ma_mutex





typedef struct
{
    ma_uint32 value
    ma_pthread_mutex_t lock
    ma_pthread_cond_t cond
} ma_event





typedef struct
{
    int value
    ma_pthread_mutex_t lock
    ma_pthread_cond_t cond
} ma_semaphore


void ma_version(ma_uint32* pMajor, ma_uint32* pMinor, ma_uint32* pRevision)
const char* ma_version_string(void)







typedef __builtin_va_list va_list
typedef __builtin_va_list __gnuc_va_list

typedef void (* ma_log_callback_proc)(void* pUserData, ma_uint32 level, const char* pMessage)

typedef struct
{
    ma_log_callback_proc onLog
    void* pUserData
} ma_log_callback

ma_log_callback ma_log_callback_init(ma_log_callback_proc onLog, void* pUserData)


typedef struct
{
    ma_log_callback callbacks[4]
    ma_uint32 callbackCount
    ma_allocation_callbacks allocationCallbacks

    ma_mutex lock

} ma_log

ma_result ma_log_init(const ma_allocation_callbacks* pAllocationCallbacks, ma_log* pLog)
void ma_log_uninit(ma_log* pLog)
ma_result ma_log_register_callback(ma_log* pLog, ma_log_callback callback)
ma_result ma_log_unregister_callback(ma_log* pLog, ma_log_callback callback)
ma_result ma_log_post(ma_log* pLog, ma_uint32 level, const char* pMessage)
ma_result ma_log_postv(ma_log* pLog, ma_uint32 level, const char* pFormat, va_list args)
ma_result ma_log_postf(ma_log* pLog, ma_uint32 level, const char* pFormat, ...) __attribute__((format(printf, 3, 4)))



typedef union
{
    float f32
    ma_int32 s32
} ma_biquad_coefficient

typedef struct
{
    ma_format format
    ma_uint32 channels
    double b0
    double b1
    double b2
    double a0
    double a1
    double a2
} ma_biquad_config

ma_biquad_config ma_biquad_config_init(ma_format format, ma_uint32 channels, double b0, double b1, double b2, double a0, double a1, double a2)

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_biquad_coefficient b0
    ma_biquad_coefficient b1
    ma_biquad_coefficient b2
    ma_biquad_coefficient a1
    ma_biquad_coefficient a2
    ma_biquad_coefficient* pR1
    ma_biquad_coefficient* pR2


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_biquad

ma_result ma_biquad_get_heap_size(const ma_biquad_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_biquad_init_preallocated(const ma_biquad_config* pConfig, void* pHeap, ma_biquad* pBQ)
ma_result ma_biquad_init(const ma_biquad_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_biquad* pBQ)
void ma_biquad_uninit(ma_biquad* pBQ, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_biquad_reinit(const ma_biquad_config* pConfig, ma_biquad* pBQ)
ma_result ma_biquad_clear_cache(ma_biquad* pBQ)
ma_result ma_biquad_process_pcm_frames(ma_biquad* pBQ, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_biquad_get_latency(const ma_biquad* pBQ)







typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double cutoffFrequency
    double q
} ma_lpf1_config, ma_lpf2_config

ma_lpf1_config ma_lpf1_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency)
ma_lpf2_config ma_lpf2_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency, double q)

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_biquad_coefficient a
    ma_biquad_coefficient* pR1


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_lpf1

ma_result ma_lpf1_get_heap_size(const ma_lpf1_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_lpf1_init_preallocated(const ma_lpf1_config* pConfig, void* pHeap, ma_lpf1* pLPF)
ma_result ma_lpf1_init(const ma_lpf1_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_lpf1* pLPF)
void ma_lpf1_uninit(ma_lpf1* pLPF, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_lpf1_reinit(const ma_lpf1_config* pConfig, ma_lpf1* pLPF)
ma_result ma_lpf1_clear_cache(ma_lpf1* pLPF)
ma_result ma_lpf1_process_pcm_frames(ma_lpf1* pLPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_lpf1_get_latency(const ma_lpf1* pLPF)

typedef struct
{
    ma_biquad bq
} ma_lpf2

ma_result ma_lpf2_get_heap_size(const ma_lpf2_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_lpf2_init_preallocated(const ma_lpf2_config* pConfig, void* pHeap, ma_lpf2* pHPF)
ma_result ma_lpf2_init(const ma_lpf2_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_lpf2* pLPF)
void ma_lpf2_uninit(ma_lpf2* pLPF, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_lpf2_reinit(const ma_lpf2_config* pConfig, ma_lpf2* pLPF)
ma_result ma_lpf2_clear_cache(ma_lpf2* pLPF)
ma_result ma_lpf2_process_pcm_frames(ma_lpf2* pLPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_lpf2_get_latency(const ma_lpf2* pLPF)


typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double cutoffFrequency
    ma_uint32 order
} ma_lpf_config

ma_lpf_config ma_lpf_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency, ma_uint32 order)

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_uint32 lpf1Count
    ma_uint32 lpf2Count
    ma_lpf1* pLPF1
    ma_lpf2* pLPF2


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_lpf

ma_result ma_lpf_get_heap_size(const ma_lpf_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_lpf_init_preallocated(const ma_lpf_config* pConfig, void* pHeap, ma_lpf* pLPF)
ma_result ma_lpf_init(const ma_lpf_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_lpf* pLPF)
void ma_lpf_uninit(ma_lpf* pLPF, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_lpf_reinit(const ma_lpf_config* pConfig, ma_lpf* pLPF)
ma_result ma_lpf_clear_cache(ma_lpf* pLPF)
ma_result ma_lpf_process_pcm_frames(ma_lpf* pLPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_lpf_get_latency(const ma_lpf* pLPF)







typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double cutoffFrequency
    double q
} ma_hpf1_config, ma_hpf2_config

ma_hpf1_config ma_hpf1_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency)
ma_hpf2_config ma_hpf2_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency, double q)

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_biquad_coefficient a
    ma_biquad_coefficient* pR1


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_hpf1

ma_result ma_hpf1_get_heap_size(const ma_hpf1_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_hpf1_init_preallocated(const ma_hpf1_config* pConfig, void* pHeap, ma_hpf1* pLPF)
ma_result ma_hpf1_init(const ma_hpf1_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_hpf1* pHPF)
void ma_hpf1_uninit(ma_hpf1* pHPF, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_hpf1_reinit(const ma_hpf1_config* pConfig, ma_hpf1* pHPF)
ma_result ma_hpf1_process_pcm_frames(ma_hpf1* pHPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_hpf1_get_latency(const ma_hpf1* pHPF)

typedef struct
{
    ma_biquad bq
} ma_hpf2

ma_result ma_hpf2_get_heap_size(const ma_hpf2_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_hpf2_init_preallocated(const ma_hpf2_config* pConfig, void* pHeap, ma_hpf2* pHPF)
ma_result ma_hpf2_init(const ma_hpf2_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_hpf2* pHPF)
void ma_hpf2_uninit(ma_hpf2* pHPF, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_hpf2_reinit(const ma_hpf2_config* pConfig, ma_hpf2* pHPF)
ma_result ma_hpf2_process_pcm_frames(ma_hpf2* pHPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_hpf2_get_latency(const ma_hpf2* pHPF)


typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double cutoffFrequency
    ma_uint32 order
} ma_hpf_config

ma_hpf_config ma_hpf_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency, ma_uint32 order)

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_uint32 hpf1Count
    ma_uint32 hpf2Count
    ma_hpf1* pHPF1
    ma_hpf2* pHPF2


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_hpf

ma_result ma_hpf_get_heap_size(const ma_hpf_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_hpf_init_preallocated(const ma_hpf_config* pConfig, void* pHeap, ma_hpf* pLPF)
ma_result ma_hpf_init(const ma_hpf_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_hpf* pHPF)
void ma_hpf_uninit(ma_hpf* pHPF, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_hpf_reinit(const ma_hpf_config* pConfig, ma_hpf* pHPF)
ma_result ma_hpf_process_pcm_frames(ma_hpf* pHPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_hpf_get_latency(const ma_hpf* pHPF)







typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double cutoffFrequency
    double q
} ma_bpf2_config

ma_bpf2_config ma_bpf2_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency, double q)

typedef struct
{
    ma_biquad bq
} ma_bpf2

ma_result ma_bpf2_get_heap_size(const ma_bpf2_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_bpf2_init_preallocated(const ma_bpf2_config* pConfig, void* pHeap, ma_bpf2* pBPF)
ma_result ma_bpf2_init(const ma_bpf2_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_bpf2* pBPF)
void ma_bpf2_uninit(ma_bpf2* pBPF, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_bpf2_reinit(const ma_bpf2_config* pConfig, ma_bpf2* pBPF)
ma_result ma_bpf2_process_pcm_frames(ma_bpf2* pBPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_bpf2_get_latency(const ma_bpf2* pBPF)


typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double cutoffFrequency
    ma_uint32 order
} ma_bpf_config

ma_bpf_config ma_bpf_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency, ma_uint32 order)

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 bpf2Count
    ma_bpf2* pBPF2


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_bpf

ma_result ma_bpf_get_heap_size(const ma_bpf_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_bpf_init_preallocated(const ma_bpf_config* pConfig, void* pHeap, ma_bpf* pBPF)
ma_result ma_bpf_init(const ma_bpf_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_bpf* pBPF)
void ma_bpf_uninit(ma_bpf* pBPF, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_bpf_reinit(const ma_bpf_config* pConfig, ma_bpf* pBPF)
ma_result ma_bpf_process_pcm_frames(ma_bpf* pBPF, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_bpf_get_latency(const ma_bpf* pBPF)







typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double q
    double frequency
} ma_notch2_config, ma_notch_config

ma_notch2_config ma_notch2_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double q, double frequency)

typedef struct
{
    ma_biquad bq
} ma_notch2

ma_result ma_notch2_get_heap_size(const ma_notch2_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_notch2_init_preallocated(const ma_notch2_config* pConfig, void* pHeap, ma_notch2* pFilter)
ma_result ma_notch2_init(const ma_notch2_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_notch2* pFilter)
void ma_notch2_uninit(ma_notch2* pFilter, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_notch2_reinit(const ma_notch2_config* pConfig, ma_notch2* pFilter)
ma_result ma_notch2_process_pcm_frames(ma_notch2* pFilter, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_notch2_get_latency(const ma_notch2* pFilter)


typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double gainDB
    double q
    double frequency
} ma_peak2_config, ma_peak_config

ma_peak2_config ma_peak2_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double gainDB, double q, double frequency)

typedef struct
{
    ma_biquad bq
} ma_peak2

ma_result ma_peak2_get_heap_size(const ma_peak2_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_peak2_init_preallocated(const ma_peak2_config* pConfig, void* pHeap, ma_peak2* pFilter)
ma_result ma_peak2_init(const ma_peak2_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_peak2* pFilter)
void ma_peak2_uninit(ma_peak2* pFilter, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_peak2_reinit(const ma_peak2_config* pConfig, ma_peak2* pFilter)
ma_result ma_peak2_process_pcm_frames(ma_peak2* pFilter, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_peak2_get_latency(const ma_peak2* pFilter)







typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double gainDB
    double shelfSlope
    double frequency
} ma_loshelf2_config, ma_loshelf_config

ma_loshelf2_config ma_loshelf2_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double gainDB, double shelfSlope, double frequency)

typedef struct
{
    ma_biquad bq
} ma_loshelf2

ma_result ma_loshelf2_get_heap_size(const ma_loshelf2_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_loshelf2_init_preallocated(const ma_loshelf2_config* pConfig, void* pHeap, ma_loshelf2* pFilter)
ma_result ma_loshelf2_init(const ma_loshelf2_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_loshelf2* pFilter)
void ma_loshelf2_uninit(ma_loshelf2* pFilter, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_loshelf2_reinit(const ma_loshelf2_config* pConfig, ma_loshelf2* pFilter)
ma_result ma_loshelf2_process_pcm_frames(ma_loshelf2* pFilter, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_loshelf2_get_latency(const ma_loshelf2* pFilter)







typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double gainDB
    double shelfSlope
    double frequency
} ma_hishelf2_config, ma_hishelf_config

ma_hishelf2_config ma_hishelf2_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double gainDB, double shelfSlope, double frequency)

typedef struct
{
    ma_biquad bq
} ma_hishelf2

ma_result ma_hishelf2_get_heap_size(const ma_hishelf2_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_hishelf2_init_preallocated(const ma_hishelf2_config* pConfig, void* pHeap, ma_hishelf2* pFilter)
ma_result ma_hishelf2_init(const ma_hishelf2_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_hishelf2* pFilter)
void ma_hishelf2_uninit(ma_hishelf2* pFilter, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_hishelf2_reinit(const ma_hishelf2_config* pConfig, ma_hishelf2* pFilter)
ma_result ma_hishelf2_process_pcm_frames(ma_hishelf2* pFilter, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_uint32 ma_hishelf2_get_latency(const ma_hishelf2* pFilter)






typedef struct
{
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_uint32 delayInFrames
    ma_bool32 delayStart
    float wet
    float dry
    float decay
} ma_delay_config

ma_delay_config ma_delay_config_init(ma_uint32 channels, ma_uint32 sampleRate, ma_uint32 delayInFrames, float decay)


typedef struct
{
    ma_delay_config config
    ma_uint32 cursor
    ma_uint32 bufferSizeInFrames
    float* pBuffer
} ma_delay

ma_result ma_delay_init(const ma_delay_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_delay* pDelay)
void ma_delay_uninit(ma_delay* pDelay, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_delay_process_pcm_frames(ma_delay* pDelay, void* pFramesOut, const void* pFramesIn, ma_uint32 frameCount)
void ma_delay_set_wet(ma_delay* pDelay, float value)
float ma_delay_get_wet(const ma_delay* pDelay)
void ma_delay_set_dry(ma_delay* pDelay, float value)
float ma_delay_get_dry(const ma_delay* pDelay)
void ma_delay_set_decay(ma_delay* pDelay, float value)
float ma_delay_get_decay(const ma_delay* pDelay)



typedef struct
{
    ma_uint32 channels
    ma_uint32 smoothTimeInFrames
} ma_gainer_config

ma_gainer_config ma_gainer_config_init(ma_uint32 channels, ma_uint32 smoothTimeInFrames)


typedef struct
{
    ma_gainer_config config
    ma_uint32 t
    float masterVolume
    float* pOldGains
    float* pNewGains


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_gainer

ma_result ma_gainer_get_heap_size(const ma_gainer_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_gainer_init_preallocated(const ma_gainer_config* pConfig, void* pHeap, ma_gainer* pGainer)
ma_result ma_gainer_init(const ma_gainer_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_gainer* pGainer)
void ma_gainer_uninit(ma_gainer* pGainer, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_gainer_process_pcm_frames(ma_gainer* pGainer, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_result ma_gainer_set_gain(ma_gainer* pGainer, float newGain)
ma_result ma_gainer_set_gains(ma_gainer* pGainer, float* pNewGains)
ma_result ma_gainer_set_master_volume(ma_gainer* pGainer, float volume)
ma_result ma_gainer_get_master_volume(const ma_gainer* pGainer, float* pVolume)




typedef enum
{
    ma_pan_mode_balance = 0,
    ma_pan_mode_pan
} ma_pan_mode

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_pan_mode mode
    float pan
} ma_panner_config

ma_panner_config ma_panner_config_init(ma_format format, ma_uint32 channels)


typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_pan_mode mode
    float pan
} ma_panner

ma_result ma_panner_init(const ma_panner_config* pConfig, ma_panner* pPanner)
ma_result ma_panner_process_pcm_frames(ma_panner* pPanner, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
void ma_panner_set_mode(ma_panner* pPanner, ma_pan_mode mode)
ma_pan_mode ma_panner_get_mode(const ma_panner* pPanner)
void ma_panner_set_pan(ma_panner* pPanner, float pan)
float ma_panner_get_pan(const ma_panner* pPanner)




typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
} ma_fader_config

ma_fader_config ma_fader_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate)

typedef struct
{
    ma_fader_config config
    float volumeBeg
    float volumeEnd
    ma_uint64 lengthInFrames
    ma_int64 cursorInFrames
} ma_fader

ma_result ma_fader_init(const ma_fader_config* pConfig, ma_fader* pFader)
ma_result ma_fader_process_pcm_frames(ma_fader* pFader, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
void ma_fader_get_data_format(const ma_fader* pFader, ma_format* pFormat, ma_uint32* pChannels, ma_uint32* pSampleRate)
void ma_fader_set_fade(ma_fader* pFader, float volumeBeg, float volumeEnd, ma_uint64 lengthInFrames)
void ma_fader_set_fade_ex(ma_fader* pFader, float volumeBeg, float volumeEnd, ma_uint64 lengthInFrames, ma_int64 startOffsetInFrames)
float ma_fader_get_current_volume(const ma_fader* pFader)




typedef struct
{
    float x
    float y
    float z
} ma_vec3f

typedef struct
{
    ma_vec3f v
    ma_spinlock lock
} ma_atomic_vec3f

typedef enum
{
    ma_attenuation_model_none,
    ma_attenuation_model_inverse,
    ma_attenuation_model_linear,
    ma_attenuation_model_exponential
} ma_attenuation_model

typedef enum
{
    ma_positioning_absolute,
    ma_positioning_relative
} ma_positioning

typedef enum
{
    ma_handedness_right,
    ma_handedness_left
} ma_handedness


typedef struct
{
    ma_uint32 channelsOut
    ma_channel* pChannelMapOut
    ma_handedness handedness
    float coneInnerAngleInRadians
    float coneOuterAngleInRadians
    float coneOuterGain
    float speedOfSound
    ma_vec3f worldUp
} ma_spatializer_listener_config

ma_spatializer_listener_config ma_spatializer_listener_config_init(ma_uint32 channelsOut)


typedef struct
{
    ma_spatializer_listener_config config
    ma_atomic_vec3f position
    ma_atomic_vec3f direction
    ma_atomic_vec3f velocity
    ma_bool32 isEnabled


    ma_bool32 _ownsHeap
    void* _pHeap
} ma_spatializer_listener

ma_result ma_spatializer_listener_get_heap_size(const ma_spatializer_listener_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_spatializer_listener_init_preallocated(const ma_spatializer_listener_config* pConfig, void* pHeap, ma_spatializer_listener* pListener)
ma_result ma_spatializer_listener_init(const ma_spatializer_listener_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_spatializer_listener* pListener)
void ma_spatializer_listener_uninit(ma_spatializer_listener* pListener, const ma_allocation_callbacks* pAllocationCallbacks)
ma_channel* ma_spatializer_listener_get_channel_map(ma_spatializer_listener* pListener)
void ma_spatializer_listener_set_cone(ma_spatializer_listener* pListener, float innerAngleInRadians, float outerAngleInRadians, float outerGain)
void ma_spatializer_listener_get_cone(const ma_spatializer_listener* pListener, float* pInnerAngleInRadians, float* pOuterAngleInRadians, float* pOuterGain)
void ma_spatializer_listener_set_position(ma_spatializer_listener* pListener, float x, float y, float z)
ma_vec3f ma_spatializer_listener_get_position(const ma_spatializer_listener* pListener)
void ma_spatializer_listener_set_direction(ma_spatializer_listener* pListener, float x, float y, float z)
ma_vec3f ma_spatializer_listener_get_direction(const ma_spatializer_listener* pListener)
void ma_spatializer_listener_set_velocity(ma_spatializer_listener* pListener, float x, float y, float z)
ma_vec3f ma_spatializer_listener_get_velocity(const ma_spatializer_listener* pListener)
void ma_spatializer_listener_set_speed_of_sound(ma_spatializer_listener* pListener, float speedOfSound)
float ma_spatializer_listener_get_speed_of_sound(const ma_spatializer_listener* pListener)
void ma_spatializer_listener_set_world_up(ma_spatializer_listener* pListener, float x, float y, float z)
ma_vec3f ma_spatializer_listener_get_world_up(const ma_spatializer_listener* pListener)
void ma_spatializer_listener_set_enabled(ma_spatializer_listener* pListener, ma_bool32 isEnabled)
ma_bool32 ma_spatializer_listener_is_enabled(const ma_spatializer_listener* pListener)


typedef struct
{
    ma_uint32 channelsIn
    ma_uint32 channelsOut
    ma_channel* pChannelMapIn
    ma_attenuation_model attenuationModel
    ma_positioning positioning
    ma_handedness handedness
    float minGain
    float maxGain
    float minDistance
    float maxDistance
    float rolloff
    float coneInnerAngleInRadians
    float coneOuterAngleInRadians
    float coneOuterGain
    float dopplerFactor
    float directionalAttenuationFactor
    float minSpatializationChannelGain
    ma_uint32 gainSmoothTimeInFrames
} ma_spatializer_config

ma_spatializer_config ma_spatializer_config_init(ma_uint32 channelsIn, ma_uint32 channelsOut)


typedef struct
{
    ma_uint32 channelsIn
    ma_uint32 channelsOut
    ma_channel* pChannelMapIn
    ma_attenuation_model attenuationModel
    ma_positioning positioning
    ma_handedness handedness
    float minGain
    float maxGain
    float minDistance
    float maxDistance
    float rolloff
    float coneInnerAngleInRadians
    float coneOuterAngleInRadians
    float coneOuterGain
    float dopplerFactor
    float directionalAttenuationFactor
    ma_uint32 gainSmoothTimeInFrames
    ma_atomic_vec3f position
    ma_atomic_vec3f direction
    ma_atomic_vec3f velocity
    float dopplerPitch
    float minSpatializationChannelGain
    ma_gainer gainer
    float* pNewChannelGainsOut


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_spatializer

ma_result ma_spatializer_get_heap_size(const ma_spatializer_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_spatializer_init_preallocated(const ma_spatializer_config* pConfig, void* pHeap, ma_spatializer* pSpatializer)
ma_result ma_spatializer_init(const ma_spatializer_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_spatializer* pSpatializer)
void ma_spatializer_uninit(ma_spatializer* pSpatializer, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_spatializer_process_pcm_frames(ma_spatializer* pSpatializer, ma_spatializer_listener* pListener, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_result ma_spatializer_set_master_volume(ma_spatializer* pSpatializer, float volume)
ma_result ma_spatializer_get_master_volume(const ma_spatializer* pSpatializer, float* pVolume)
ma_uint32 ma_spatializer_get_input_channels(const ma_spatializer* pSpatializer)
ma_uint32 ma_spatializer_get_output_channels(const ma_spatializer* pSpatializer)
void ma_spatializer_set_attenuation_model(ma_spatializer* pSpatializer, ma_attenuation_model attenuationModel)
ma_attenuation_model ma_spatializer_get_attenuation_model(const ma_spatializer* pSpatializer)
void ma_spatializer_set_positioning(ma_spatializer* pSpatializer, ma_positioning positioning)
ma_positioning ma_spatializer_get_positioning(const ma_spatializer* pSpatializer)
void ma_spatializer_set_rolloff(ma_spatializer* pSpatializer, float rolloff)
float ma_spatializer_get_rolloff(const ma_spatializer* pSpatializer)
void ma_spatializer_set_min_gain(ma_spatializer* pSpatializer, float minGain)
float ma_spatializer_get_min_gain(const ma_spatializer* pSpatializer)
void ma_spatializer_set_max_gain(ma_spatializer* pSpatializer, float maxGain)
float ma_spatializer_get_max_gain(const ma_spatializer* pSpatializer)
void ma_spatializer_set_min_distance(ma_spatializer* pSpatializer, float minDistance)
float ma_spatializer_get_min_distance(const ma_spatializer* pSpatializer)
void ma_spatializer_set_max_distance(ma_spatializer* pSpatializer, float maxDistance)
float ma_spatializer_get_max_distance(const ma_spatializer* pSpatializer)
void ma_spatializer_set_cone(ma_spatializer* pSpatializer, float innerAngleInRadians, float outerAngleInRadians, float outerGain)
void ma_spatializer_get_cone(const ma_spatializer* pSpatializer, float* pInnerAngleInRadians, float* pOuterAngleInRadians, float* pOuterGain)
void ma_spatializer_set_doppler_factor(ma_spatializer* pSpatializer, float dopplerFactor)
float ma_spatializer_get_doppler_factor(const ma_spatializer* pSpatializer)
void ma_spatializer_set_directional_attenuation_factor(ma_spatializer* pSpatializer, float directionalAttenuationFactor)
float ma_spatializer_get_directional_attenuation_factor(const ma_spatializer* pSpatializer)
void ma_spatializer_set_position(ma_spatializer* pSpatializer, float x, float y, float z)
ma_vec3f ma_spatializer_get_position(const ma_spatializer* pSpatializer)
void ma_spatializer_set_direction(ma_spatializer* pSpatializer, float x, float y, float z)
ma_vec3f ma_spatializer_get_direction(const ma_spatializer* pSpatializer)
void ma_spatializer_set_velocity(ma_spatializer* pSpatializer, float x, float y, float z)
ma_vec3f ma_spatializer_get_velocity(const ma_spatializer* pSpatializer)
void ma_spatializer_get_relative_position_and_direction(const ma_spatializer* pSpatializer, const ma_spatializer_listener* pListener, ma_vec3f* pRelativePos, ma_vec3f* pRelativeDir)

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRateIn
    ma_uint32 sampleRateOut
    ma_uint32 lpfOrder
    double lpfNyquistFactor
} ma_linear_resampler_config

ma_linear_resampler_config ma_linear_resampler_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRateIn, ma_uint32 sampleRateOut)

typedef struct
{
    ma_linear_resampler_config config
    ma_uint32 inAdvanceInt
    ma_uint32 inAdvanceFrac
    ma_uint32 inTimeInt
    ma_uint32 inTimeFrac
    union
    {
        float* f32
        ma_int16* s16
    } x0
    union
    {
        float* f32
        ma_int16* s16
    } x1
    ma_lpf lpf


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_linear_resampler

ma_result ma_linear_resampler_get_heap_size(const ma_linear_resampler_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_linear_resampler_init_preallocated(const ma_linear_resampler_config* pConfig, void* pHeap, ma_linear_resampler* pResampler)
ma_result ma_linear_resampler_init(const ma_linear_resampler_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_linear_resampler* pResampler)
void ma_linear_resampler_uninit(ma_linear_resampler* pResampler, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_linear_resampler_process_pcm_frames(ma_linear_resampler* pResampler, const void* pFramesIn, ma_uint64* pFrameCountIn, void* pFramesOut, ma_uint64* pFrameCountOut)
ma_result ma_linear_resampler_set_rate(ma_linear_resampler* pResampler, ma_uint32 sampleRateIn, ma_uint32 sampleRateOut)
ma_result ma_linear_resampler_set_rate_ratio(ma_linear_resampler* pResampler, float ratioInOut)
ma_uint64 ma_linear_resampler_get_input_latency(const ma_linear_resampler* pResampler)
ma_uint64 ma_linear_resampler_get_output_latency(const ma_linear_resampler* pResampler)
ma_result ma_linear_resampler_get_required_input_frame_count(const ma_linear_resampler* pResampler, ma_uint64 outputFrameCount, ma_uint64* pInputFrameCount)
ma_result ma_linear_resampler_get_expected_output_frame_count(const ma_linear_resampler* pResampler, ma_uint64 inputFrameCount, ma_uint64* pOutputFrameCount)
ma_result ma_linear_resampler_reset(ma_linear_resampler* pResampler)


typedef struct ma_resampler_config ma_resampler_config

typedef void ma_resampling_backend
typedef struct
{
    ma_result (* onGetHeapSize )(void* pUserData, const ma_resampler_config* pConfig, size_t* pHeapSizeInBytes)
    ma_result (* onInit )(void* pUserData, const ma_resampler_config* pConfig, void* pHeap, ma_resampling_backend** ppBackend)
    void (* onUninit )(void* pUserData, ma_resampling_backend* pBackend, const ma_allocation_callbacks* pAllocationCallbacks)
    ma_result (* onProcess )(void* pUserData, ma_resampling_backend* pBackend, const void* pFramesIn, ma_uint64* pFrameCountIn, void* pFramesOut, ma_uint64* pFrameCountOut)
    ma_result (* onSetRate )(void* pUserData, ma_resampling_backend* pBackend, ma_uint32 sampleRateIn, ma_uint32 sampleRateOut)
    ma_uint64 (* onGetInputLatency )(void* pUserData, const ma_resampling_backend* pBackend)
    ma_uint64 (* onGetOutputLatency )(void* pUserData, const ma_resampling_backend* pBackend)
    ma_result (* onGetRequiredInputFrameCount )(void* pUserData, const ma_resampling_backend* pBackend, ma_uint64 outputFrameCount, ma_uint64* pInputFrameCount)
    ma_result (* onGetExpectedOutputFrameCount)(void* pUserData, const ma_resampling_backend* pBackend, ma_uint64 inputFrameCount, ma_uint64* pOutputFrameCount)
    ma_result (* onReset )(void* pUserData, ma_resampling_backend* pBackend)
} ma_resampling_backend_vtable

typedef enum
{
    ma_resample_algorithm_linear = 0,
    ma_resample_algorithm_custom,
} ma_resample_algorithm

struct ma_resampler_config
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRateIn
    ma_uint32 sampleRateOut
    ma_resample_algorithm algorithm
    ma_resampling_backend_vtable* pBackendVTable
    void* pBackendUserData
    struct
    {
        ma_uint32 lpfOrder
    } linear
}

ma_resampler_config ma_resampler_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRateIn, ma_uint32 sampleRateOut, ma_resample_algorithm algorithm)

typedef struct
{
    ma_resampling_backend* pBackend
    ma_resampling_backend_vtable* pBackendVTable
    void* pBackendUserData
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRateIn
    ma_uint32 sampleRateOut
    union
    {
        ma_linear_resampler linear
    } state


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_resampler

ma_result ma_resampler_get_heap_size(const ma_resampler_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_resampler_init_preallocated(const ma_resampler_config* pConfig, void* pHeap, ma_resampler* pResampler)




ma_result ma_resampler_init(const ma_resampler_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_resampler* pResampler)




void ma_resampler_uninit(ma_resampler* pResampler, const ma_allocation_callbacks* pAllocationCallbacks)

ma_result ma_resampler_process_pcm_frames(ma_resampler* pResampler, const void* pFramesIn, ma_uint64* pFrameCountIn, void* pFramesOut, ma_uint64* pFrameCountOut)





ma_result ma_resampler_set_rate(ma_resampler* pResampler, ma_uint32 sampleRateIn, ma_uint32 sampleRateOut)






ma_result ma_resampler_set_rate_ratio(ma_resampler* pResampler, float ratio)




ma_uint64 ma_resampler_get_input_latency(const ma_resampler* pResampler)




ma_uint64 ma_resampler_get_output_latency(const ma_resampler* pResampler)

ma_result ma_resampler_get_required_input_frame_count(const ma_resampler* pResampler, ma_uint64 outputFrameCount, ma_uint64* pInputFrameCount)





ma_result ma_resampler_get_expected_output_frame_count(const ma_resampler* pResampler, ma_uint64 inputFrameCount, ma_uint64* pOutputFrameCount)




ma_result ma_resampler_reset(ma_resampler* pResampler)







typedef enum
{
    ma_channel_conversion_path_unknown,
    ma_channel_conversion_path_passthrough,
    ma_channel_conversion_path_mono_out,
    ma_channel_conversion_path_mono_in,
    ma_channel_conversion_path_shuffle,
    ma_channel_conversion_path_weights
} ma_channel_conversion_path

typedef enum
{
    ma_mono_expansion_mode_duplicate = 0,
    ma_mono_expansion_mode_average,
    ma_mono_expansion_mode_stereo_only,
    ma_mono_expansion_mode_default = ma_mono_expansion_mode_duplicate
} ma_mono_expansion_mode

typedef struct
{
    ma_format format
    ma_uint32 channelsIn
    ma_uint32 channelsOut
    const ma_channel* pChannelMapIn
    const ma_channel* pChannelMapOut
    ma_channel_mix_mode mixingMode
    ma_bool32 calculateLFEFromSpatialChannels
    float** ppWeights
} ma_channel_converter_config

ma_channel_converter_config ma_channel_converter_config_init(ma_format format, ma_uint32 channelsIn, const ma_channel* pChannelMapIn, ma_uint32 channelsOut, const ma_channel* pChannelMapOut, ma_channel_mix_mode mixingMode)

typedef struct
{
    ma_format format
    ma_uint32 channelsIn
    ma_uint32 channelsOut
    ma_channel_mix_mode mixingMode
    ma_channel_conversion_path conversionPath
    ma_channel* pChannelMapIn
    ma_channel* pChannelMapOut
    ma_uint8* pShuffleTable
    union
    {
        float** f32
        ma_int32** s16
    } weights


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_channel_converter

ma_result ma_channel_converter_get_heap_size(const ma_channel_converter_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_channel_converter_init_preallocated(const ma_channel_converter_config* pConfig, void* pHeap, ma_channel_converter* pConverter)
ma_result ma_channel_converter_init(const ma_channel_converter_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_channel_converter* pConverter)
void ma_channel_converter_uninit(ma_channel_converter* pConverter, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_channel_converter_process_pcm_frames(ma_channel_converter* pConverter, void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount)
ma_result ma_channel_converter_get_input_channel_map(const ma_channel_converter* pConverter, ma_channel* pChannelMap, size_t channelMapCap)
ma_result ma_channel_converter_get_output_channel_map(const ma_channel_converter* pConverter, ma_channel* pChannelMap, size_t channelMapCap)







typedef struct
{
    ma_format formatIn
    ma_format formatOut
    ma_uint32 channelsIn
    ma_uint32 channelsOut
    ma_uint32 sampleRateIn
    ma_uint32 sampleRateOut
    ma_channel* pChannelMapIn
    ma_channel* pChannelMapOut
    ma_dither_mode ditherMode
    ma_channel_mix_mode channelMixMode
    ma_bool32 calculateLFEFromSpatialChannels
    float** ppChannelWeights
    ma_bool32 allowDynamicSampleRate
    ma_resampler_config resampling
} ma_data_converter_config

ma_data_converter_config ma_data_converter_config_init_default(void)
ma_data_converter_config ma_data_converter_config_init(ma_format formatIn, ma_format formatOut, ma_uint32 channelsIn, ma_uint32 channelsOut, ma_uint32 sampleRateIn, ma_uint32 sampleRateOut)


typedef enum
{
    ma_data_converter_execution_path_passthrough,
    ma_data_converter_execution_path_format_only,
    ma_data_converter_execution_path_channels_only,
    ma_data_converter_execution_path_resample_only,
    ma_data_converter_execution_path_resample_first,
    ma_data_converter_execution_path_channels_first
} ma_data_converter_execution_path

typedef struct
{
    ma_format formatIn
    ma_format formatOut
    ma_uint32 channelsIn
    ma_uint32 channelsOut
    ma_uint32 sampleRateIn
    ma_uint32 sampleRateOut
    ma_dither_mode ditherMode
    ma_data_converter_execution_path executionPath
    ma_channel_converter channelConverter
    ma_resampler resampler
    ma_bool8 hasPreFormatConversion
    ma_bool8 hasPostFormatConversion
    ma_bool8 hasChannelConverter
    ma_bool8 hasResampler
    ma_bool8 isPassthrough


    ma_bool8 _ownsHeap
    void* _pHeap
} ma_data_converter

ma_result ma_data_converter_get_heap_size(const ma_data_converter_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_data_converter_init_preallocated(const ma_data_converter_config* pConfig, void* pHeap, ma_data_converter* pConverter)
ma_result ma_data_converter_init(const ma_data_converter_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_data_converter* pConverter)
void ma_data_converter_uninit(ma_data_converter* pConverter, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_data_converter_process_pcm_frames(ma_data_converter* pConverter, const void* pFramesIn, ma_uint64* pFrameCountIn, void* pFramesOut, ma_uint64* pFrameCountOut)
ma_result ma_data_converter_set_rate(ma_data_converter* pConverter, ma_uint32 sampleRateIn, ma_uint32 sampleRateOut)
ma_result ma_data_converter_set_rate_ratio(ma_data_converter* pConverter, float ratioInOut)
ma_uint64 ma_data_converter_get_input_latency(const ma_data_converter* pConverter)
ma_uint64 ma_data_converter_get_output_latency(const ma_data_converter* pConverter)
ma_result ma_data_converter_get_required_input_frame_count(const ma_data_converter* pConverter, ma_uint64 outputFrameCount, ma_uint64* pInputFrameCount)
ma_result ma_data_converter_get_expected_output_frame_count(const ma_data_converter* pConverter, ma_uint64 inputFrameCount, ma_uint64* pOutputFrameCount)
ma_result ma_data_converter_get_input_channel_map(const ma_data_converter* pConverter, ma_channel* pChannelMap, size_t channelMapCap)
ma_result ma_data_converter_get_output_channel_map(const ma_data_converter* pConverter, ma_channel* pChannelMap, size_t channelMapCap)
ma_result ma_data_converter_reset(ma_data_converter* pConverter)







void ma_pcm_u8_to_s16(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_u8_to_s24(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_u8_to_s32(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_u8_to_f32(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s16_to_u8(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s16_to_s24(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s16_to_s32(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s16_to_f32(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s24_to_u8(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s24_to_s16(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s24_to_s32(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s24_to_f32(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s32_to_u8(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s32_to_s16(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s32_to_s24(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_s32_to_f32(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_f32_to_u8(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_f32_to_s16(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_f32_to_s24(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_f32_to_s32(void* pOut, const void* pIn, ma_uint64 count, ma_dither_mode ditherMode)
void ma_pcm_convert(void* pOut, ma_format formatOut, const void* pIn, ma_format formatIn, ma_uint64 sampleCount, ma_dither_mode ditherMode)
void ma_convert_pcm_frames_format(void* pOut, ma_format formatOut, const void* pIn, ma_format formatIn, ma_uint64 frameCount, ma_uint32 channels, ma_dither_mode ditherMode)




void ma_deinterleave_pcm_frames(ma_format format, ma_uint32 channels, ma_uint64 frameCount, const void* pInterleavedPCMFrames, void** ppDeinterleavedPCMFrames)




void ma_interleave_pcm_frames(ma_format format, ma_uint32 channels, ma_uint64 frameCount, const void** ppDeinterleavedPCMFrames, void* pInterleavedPCMFrames)

ma_channel ma_channel_map_get_channel(const ma_channel* pChannelMap, ma_uint32 channelCount, ma_uint32 channelIndex)






void ma_channel_map_init_blank(ma_channel* pChannelMap, ma_uint32 channels)






void ma_channel_map_init_standard(ma_standard_channel_map standardChannelMap, ma_channel* pChannelMap, size_t channelMapCap, ma_uint32 channels)






void ma_channel_map_copy(ma_channel* pOut, const ma_channel* pIn, ma_uint32 channels)






void ma_channel_map_copy_or_default(ma_channel* pOut, size_t channelMapCapOut, const ma_channel* pIn, ma_uint32 channels)

ma_bool32 ma_channel_map_is_valid(const ma_channel* pChannelMap, ma_uint32 channels)

ma_bool32 ma_channel_map_is_equal(const ma_channel* pChannelMapA, const ma_channel* pChannelMapB, ma_uint32 channels)






ma_bool32 ma_channel_map_is_blank(const ma_channel* pChannelMap, ma_uint32 channels)






ma_bool32 ma_channel_map_contains_channel_position(ma_uint32 channels, const ma_channel* pChannelMap, ma_channel channelPosition)







ma_bool32 ma_channel_map_find_channel_position(ma_uint32 channels, const ma_channel* pChannelMap, ma_channel channelPosition, ma_uint32* pChannelIndex)

size_t ma_channel_map_to_string(const ma_channel* pChannelMap, ma_uint32 channels, char* pBufferOut, size_t bufferCap)




const char* ma_channel_position_to_string(ma_channel channel)

ma_uint64 ma_convert_frames(void* pOut, ma_uint64 frameCountOut, ma_format formatOut, ma_uint32 channelsOut, ma_uint32 sampleRateOut, const void* pIn, ma_uint64 frameCountIn, ma_format formatIn, ma_uint32 channelsIn, ma_uint32 sampleRateIn)
ma_uint64 ma_convert_frames_ex(void* pOut, ma_uint64 frameCountOut, const void* pIn, ma_uint64 frameCountIn, const ma_data_converter_config* pConfig)







typedef void ma_data_source



typedef struct
{
    ma_result (* onRead)(ma_data_source* pDataSource, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
    ma_result (* onSeek)(ma_data_source* pDataSource, ma_uint64 frameIndex)
    ma_result (* onGetDataFormat)(ma_data_source* pDataSource, ma_format* pFormat, ma_uint32* pChannels, ma_uint32* pSampleRate, ma_channel* pChannelMap, size_t channelMapCap)
    ma_result (* onGetCursor)(ma_data_source* pDataSource, ma_uint64* pCursor)
    ma_result (* onGetLength)(ma_data_source* pDataSource, ma_uint64* pLength)
    ma_result (* onSetLooping)(ma_data_source* pDataSource, ma_bool32 isLooping)
    ma_uint32 flags
} ma_data_source_vtable

typedef ma_data_source* (* ma_data_source_get_next_proc)(ma_data_source* pDataSource)

typedef struct
{
    const ma_data_source_vtable* vtable
} ma_data_source_config

ma_data_source_config ma_data_source_config_init(void)


typedef struct
{
    const ma_data_source_vtable* vtable
    ma_uint64 rangeBegInFrames
    ma_uint64 rangeEndInFrames
    ma_uint64 loopBegInFrames
    ma_uint64 loopEndInFrames
    ma_data_source* pCurrent
    ma_data_source* pNext
    ma_data_source_get_next_proc onGetNext
    _Alignas(4) ma_bool32 isLooping
} ma_data_source_base

ma_result ma_data_source_init(const ma_data_source_config* pConfig, ma_data_source* pDataSource)
void ma_data_source_uninit(ma_data_source* pDataSource)
ma_result ma_data_source_read_pcm_frames(ma_data_source* pDataSource, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_result ma_data_source_seek_pcm_frames(ma_data_source* pDataSource, ma_uint64 frameCount, ma_uint64* pFramesSeeked)
ma_result ma_data_source_seek_to_pcm_frame(ma_data_source* pDataSource, ma_uint64 frameIndex)
ma_result ma_data_source_get_data_format(ma_data_source* pDataSource, ma_format* pFormat, ma_uint32* pChannels, ma_uint32* pSampleRate, ma_channel* pChannelMap, size_t channelMapCap)
ma_result ma_data_source_get_cursor_in_pcm_frames(ma_data_source* pDataSource, ma_uint64* pCursor)
ma_result ma_data_source_get_length_in_pcm_frames(ma_data_source* pDataSource, ma_uint64* pLength)
ma_result ma_data_source_get_cursor_in_seconds(ma_data_source* pDataSource, float* pCursor)
ma_result ma_data_source_get_length_in_seconds(ma_data_source* pDataSource, float* pLength)
ma_result ma_data_source_set_looping(ma_data_source* pDataSource, ma_bool32 isLooping)
ma_bool32 ma_data_source_is_looping(const ma_data_source* pDataSource)
ma_result ma_data_source_set_range_in_pcm_frames(ma_data_source* pDataSource, ma_uint64 rangeBegInFrames, ma_uint64 rangeEndInFrames)
void ma_data_source_get_range_in_pcm_frames(const ma_data_source* pDataSource, ma_uint64* pRangeBegInFrames, ma_uint64* pRangeEndInFrames)
ma_result ma_data_source_set_loop_point_in_pcm_frames(ma_data_source* pDataSource, ma_uint64 loopBegInFrames, ma_uint64 loopEndInFrames)
void ma_data_source_get_loop_point_in_pcm_frames(const ma_data_source* pDataSource, ma_uint64* pLoopBegInFrames, ma_uint64* pLoopEndInFrames)
ma_result ma_data_source_set_current(ma_data_source* pDataSource, ma_data_source* pCurrentDataSource)
ma_data_source* ma_data_source_get_current(const ma_data_source* pDataSource)
ma_result ma_data_source_set_next(ma_data_source* pDataSource, ma_data_source* pNextDataSource)
ma_data_source* ma_data_source_get_next(const ma_data_source* pDataSource)
ma_result ma_data_source_set_next_callback(ma_data_source* pDataSource, ma_data_source_get_next_proc onGetNext)
ma_data_source_get_next_proc ma_data_source_get_next_callback(const ma_data_source* pDataSource)


typedef struct
{
    ma_data_source_base ds
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_uint64 cursor
    ma_uint64 sizeInFrames
    const void* pData
} ma_audio_buffer_ref

ma_result ma_audio_buffer_ref_init(ma_format format, ma_uint32 channels, const void* pData, ma_uint64 sizeInFrames, ma_audio_buffer_ref* pAudioBufferRef)
void ma_audio_buffer_ref_uninit(ma_audio_buffer_ref* pAudioBufferRef)
ma_result ma_audio_buffer_ref_set_data(ma_audio_buffer_ref* pAudioBufferRef, const void* pData, ma_uint64 sizeInFrames)
ma_uint64 ma_audio_buffer_ref_read_pcm_frames(ma_audio_buffer_ref* pAudioBufferRef, void* pFramesOut, ma_uint64 frameCount, ma_bool32 loop)
ma_result ma_audio_buffer_ref_seek_to_pcm_frame(ma_audio_buffer_ref* pAudioBufferRef, ma_uint64 frameIndex)
ma_result ma_audio_buffer_ref_map(ma_audio_buffer_ref* pAudioBufferRef, void** ppFramesOut, ma_uint64* pFrameCount)
ma_result ma_audio_buffer_ref_unmap(ma_audio_buffer_ref* pAudioBufferRef, ma_uint64 frameCount)
ma_bool32 ma_audio_buffer_ref_at_end(const ma_audio_buffer_ref* pAudioBufferRef)
ma_result ma_audio_buffer_ref_get_cursor_in_pcm_frames(const ma_audio_buffer_ref* pAudioBufferRef, ma_uint64* pCursor)
ma_result ma_audio_buffer_ref_get_length_in_pcm_frames(const ma_audio_buffer_ref* pAudioBufferRef, ma_uint64* pLength)
ma_result ma_audio_buffer_ref_get_available_frames(const ma_audio_buffer_ref* pAudioBufferRef, ma_uint64* pAvailableFrames)



typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_uint64 sizeInFrames
    const void* pData
    ma_allocation_callbacks allocationCallbacks
} ma_audio_buffer_config

ma_audio_buffer_config ma_audio_buffer_config_init(ma_format format, ma_uint32 channels, ma_uint64 sizeInFrames, const void* pData, const ma_allocation_callbacks* pAllocationCallbacks)

typedef struct
{
    ma_audio_buffer_ref ref
    ma_allocation_callbacks allocationCallbacks
    ma_bool32 ownsData
    ma_uint8 _pExtraData[1]
} ma_audio_buffer

ma_result ma_audio_buffer_init(const ma_audio_buffer_config* pConfig, ma_audio_buffer* pAudioBuffer)
ma_result ma_audio_buffer_init_copy(const ma_audio_buffer_config* pConfig, ma_audio_buffer* pAudioBuffer)
ma_result ma_audio_buffer_alloc_and_init(const ma_audio_buffer_config* pConfig, ma_audio_buffer** ppAudioBuffer)
void ma_audio_buffer_uninit(ma_audio_buffer* pAudioBuffer)
void ma_audio_buffer_uninit_and_free(ma_audio_buffer* pAudioBuffer)
ma_uint64 ma_audio_buffer_read_pcm_frames(ma_audio_buffer* pAudioBuffer, void* pFramesOut, ma_uint64 frameCount, ma_bool32 loop)
ma_result ma_audio_buffer_seek_to_pcm_frame(ma_audio_buffer* pAudioBuffer, ma_uint64 frameIndex)
ma_result ma_audio_buffer_map(ma_audio_buffer* pAudioBuffer, void** ppFramesOut, ma_uint64* pFrameCount)
ma_result ma_audio_buffer_unmap(ma_audio_buffer* pAudioBuffer, ma_uint64 frameCount)
ma_bool32 ma_audio_buffer_at_end(const ma_audio_buffer* pAudioBuffer)
ma_result ma_audio_buffer_get_cursor_in_pcm_frames(const ma_audio_buffer* pAudioBuffer, ma_uint64* pCursor)
ma_result ma_audio_buffer_get_length_in_pcm_frames(const ma_audio_buffer* pAudioBuffer, ma_uint64* pLength)
ma_result ma_audio_buffer_get_available_frames(const ma_audio_buffer* pAudioBuffer, ma_uint64* pAvailableFrames)

typedef struct ma_paged_audio_buffer_page ma_paged_audio_buffer_page
struct ma_paged_audio_buffer_page
{
    _Alignas(8) ma_paged_audio_buffer_page* pNext
    ma_uint64 sizeInFrames
    ma_uint8 pAudioData[1]
}

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_paged_audio_buffer_page head
    _Alignas(8) ma_paged_audio_buffer_page* pTail
} ma_paged_audio_buffer_data

ma_result ma_paged_audio_buffer_data_init(ma_format format, ma_uint32 channels, ma_paged_audio_buffer_data* pData)
void ma_paged_audio_buffer_data_uninit(ma_paged_audio_buffer_data* pData, const ma_allocation_callbacks* pAllocationCallbacks)
ma_paged_audio_buffer_page* ma_paged_audio_buffer_data_get_head(ma_paged_audio_buffer_data* pData)
ma_paged_audio_buffer_page* ma_paged_audio_buffer_data_get_tail(ma_paged_audio_buffer_data* pData)
ma_result ma_paged_audio_buffer_data_get_length_in_pcm_frames(ma_paged_audio_buffer_data* pData, ma_uint64* pLength)
ma_result ma_paged_audio_buffer_data_allocate_page(ma_paged_audio_buffer_data* pData, ma_uint64 pageSizeInFrames, const void* pInitialData, const ma_allocation_callbacks* pAllocationCallbacks, ma_paged_audio_buffer_page** ppPage)
ma_result ma_paged_audio_buffer_data_free_page(ma_paged_audio_buffer_data* pData, ma_paged_audio_buffer_page* pPage, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_paged_audio_buffer_data_append_page(ma_paged_audio_buffer_data* pData, ma_paged_audio_buffer_page* pPage)
ma_result ma_paged_audio_buffer_data_allocate_and_append_page(ma_paged_audio_buffer_data* pData, ma_uint32 pageSizeInFrames, const void* pInitialData, const ma_allocation_callbacks* pAllocationCallbacks)


typedef struct
{
    ma_paged_audio_buffer_data* pData
} ma_paged_audio_buffer_config

ma_paged_audio_buffer_config ma_paged_audio_buffer_config_init(ma_paged_audio_buffer_data* pData)


typedef struct
{
    ma_data_source_base ds
    ma_paged_audio_buffer_data* pData
    ma_paged_audio_buffer_page* pCurrent
    ma_uint64 relativeCursor
    ma_uint64 absoluteCursor
} ma_paged_audio_buffer

ma_result ma_paged_audio_buffer_init(const ma_paged_audio_buffer_config* pConfig, ma_paged_audio_buffer* pPagedAudioBuffer)
void ma_paged_audio_buffer_uninit(ma_paged_audio_buffer* pPagedAudioBuffer)
ma_result ma_paged_audio_buffer_read_pcm_frames(ma_paged_audio_buffer* pPagedAudioBuffer, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_result ma_paged_audio_buffer_seek_to_pcm_frame(ma_paged_audio_buffer* pPagedAudioBuffer, ma_uint64 frameIndex)
ma_result ma_paged_audio_buffer_get_cursor_in_pcm_frames(ma_paged_audio_buffer* pPagedAudioBuffer, ma_uint64* pCursor)
ma_result ma_paged_audio_buffer_get_length_in_pcm_frames(ma_paged_audio_buffer* pPagedAudioBuffer, ma_uint64* pLength)

typedef struct
{
    void* pBuffer
    ma_uint32 subbufferSizeInBytes
    ma_uint32 subbufferCount
    ma_uint32 subbufferStrideInBytes
    _Alignas(4) ma_uint32 encodedReadOffset
    _Alignas(4) ma_uint32 encodedWriteOffset
    ma_bool8 ownsBuffer
    ma_bool8 clearOnWriteAcquire
    ma_allocation_callbacks allocationCallbacks
} ma_rb

ma_result ma_rb_init_ex(size_t subbufferSizeInBytes, size_t subbufferCount, size_t subbufferStrideInBytes, void* pOptionalPreallocatedBuffer, const ma_allocation_callbacks* pAllocationCallbacks, ma_rb* pRB)
ma_result ma_rb_init(size_t bufferSizeInBytes, void* pOptionalPreallocatedBuffer, const ma_allocation_callbacks* pAllocationCallbacks, ma_rb* pRB)
void ma_rb_uninit(ma_rb* pRB)
void ma_rb_reset(ma_rb* pRB)
ma_result ma_rb_acquire_read(ma_rb* pRB, size_t* pSizeInBytes, void** ppBufferOut)
ma_result ma_rb_commit_read(ma_rb* pRB, size_t sizeInBytes)
ma_result ma_rb_acquire_write(ma_rb* pRB, size_t* pSizeInBytes, void** ppBufferOut)
ma_result ma_rb_commit_write(ma_rb* pRB, size_t sizeInBytes)
ma_result ma_rb_seek_read(ma_rb* pRB, size_t offsetInBytes)
ma_result ma_rb_seek_write(ma_rb* pRB, size_t offsetInBytes)
ma_int32 ma_rb_pointer_distance(ma_rb* pRB)
ma_uint32 ma_rb_available_read(ma_rb* pRB)
ma_uint32 ma_rb_available_write(ma_rb* pRB)
size_t ma_rb_get_subbuffer_size(ma_rb* pRB)
size_t ma_rb_get_subbuffer_stride(ma_rb* pRB)
size_t ma_rb_get_subbuffer_offset(ma_rb* pRB, size_t subbufferIndex)
void* ma_rb_get_subbuffer_ptr(ma_rb* pRB, size_t subbufferIndex, void* pBuffer)


typedef struct
{
    ma_data_source_base ds
    ma_rb rb
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
} ma_pcm_rb

ma_result ma_pcm_rb_init_ex(ma_format format, ma_uint32 channels, ma_uint32 subbufferSizeInFrames, ma_uint32 subbufferCount, ma_uint32 subbufferStrideInFrames, void* pOptionalPreallocatedBuffer, const ma_allocation_callbacks* pAllocationCallbacks, ma_pcm_rb* pRB)
ma_result ma_pcm_rb_init(ma_format format, ma_uint32 channels, ma_uint32 bufferSizeInFrames, void* pOptionalPreallocatedBuffer, const ma_allocation_callbacks* pAllocationCallbacks, ma_pcm_rb* pRB)
void ma_pcm_rb_uninit(ma_pcm_rb* pRB)
void ma_pcm_rb_reset(ma_pcm_rb* pRB)
ma_result ma_pcm_rb_acquire_read(ma_pcm_rb* pRB, ma_uint32* pSizeInFrames, void** ppBufferOut)
ma_result ma_pcm_rb_commit_read(ma_pcm_rb* pRB, ma_uint32 sizeInFrames)
ma_result ma_pcm_rb_acquire_write(ma_pcm_rb* pRB, ma_uint32* pSizeInFrames, void** ppBufferOut)
ma_result ma_pcm_rb_commit_write(ma_pcm_rb* pRB, ma_uint32 sizeInFrames)
ma_result ma_pcm_rb_seek_read(ma_pcm_rb* pRB, ma_uint32 offsetInFrames)
ma_result ma_pcm_rb_seek_write(ma_pcm_rb* pRB, ma_uint32 offsetInFrames)
ma_int32 ma_pcm_rb_pointer_distance(ma_pcm_rb* pRB)
ma_uint32 ma_pcm_rb_available_read(ma_pcm_rb* pRB)
ma_uint32 ma_pcm_rb_available_write(ma_pcm_rb* pRB)
ma_uint32 ma_pcm_rb_get_subbuffer_size(ma_pcm_rb* pRB)
ma_uint32 ma_pcm_rb_get_subbuffer_stride(ma_pcm_rb* pRB)
ma_uint32 ma_pcm_rb_get_subbuffer_offset(ma_pcm_rb* pRB, ma_uint32 subbufferIndex)
void* ma_pcm_rb_get_subbuffer_ptr(ma_pcm_rb* pRB, ma_uint32 subbufferIndex, void* pBuffer)
ma_format ma_pcm_rb_get_format(const ma_pcm_rb* pRB)
ma_uint32 ma_pcm_rb_get_channels(const ma_pcm_rb* pRB)
ma_uint32 ma_pcm_rb_get_sample_rate(const ma_pcm_rb* pRB)
void ma_pcm_rb_set_sample_rate(ma_pcm_rb* pRB, ma_uint32 sampleRate)


typedef struct
{
    ma_pcm_rb rb
} ma_duplex_rb

ma_result ma_duplex_rb_init(ma_format captureFormat, ma_uint32 captureChannels, ma_uint32 sampleRate, ma_uint32 captureInternalSampleRate, ma_uint32 captureInternalPeriodSizeInFrames, const ma_allocation_callbacks* pAllocationCallbacks, ma_duplex_rb* pRB)
ma_result ma_duplex_rb_uninit(ma_duplex_rb* pRB)

const char* ma_result_description(ma_result result)




void* ma_malloc(size_t sz, const ma_allocation_callbacks* pAllocationCallbacks)




void* ma_calloc(size_t sz, const ma_allocation_callbacks* pAllocationCallbacks)




void* ma_realloc(void* p, size_t sz, const ma_allocation_callbacks* pAllocationCallbacks)




void ma_free(void* p, const ma_allocation_callbacks* pAllocationCallbacks)




void* ma_aligned_malloc(size_t sz, size_t alignment, const ma_allocation_callbacks* pAllocationCallbacks)




void ma_aligned_free(void* p, const ma_allocation_callbacks* pAllocationCallbacks)




const char* ma_get_format_name(ma_format format)




void ma_blend_f32(float* pOut, float* pInA, float* pInB, float factor, ma_uint32 channels)

ma_uint32 ma_get_bytes_per_sample(ma_format format)
static inline __attribute__((always_inline)) ma_uint32 ma_get_bytes_per_frame(ma_format format, ma_uint32 channels) { return ma_get_bytes_per_sample(format) * channels }




const char* ma_log_level_to_string(ma_uint32 logLevel)

ma_result ma_spinlock_lock(volatile ma_spinlock* pSpinlock)




ma_result ma_spinlock_lock_noyield(volatile ma_spinlock* pSpinlock)




ma_result ma_spinlock_unlock(volatile ma_spinlock* pSpinlock)

ma_result ma_mutex_init(ma_mutex* pMutex)




void ma_mutex_uninit(ma_mutex* pMutex)




void ma_mutex_lock(ma_mutex* pMutex)




void ma_mutex_unlock(ma_mutex* pMutex)





ma_result ma_event_init(ma_event* pEvent)




void ma_event_uninit(ma_event* pEvent)




ma_result ma_event_wait(ma_event* pEvent)




ma_result ma_event_signal(ma_event* pEvent)

typedef struct
{

    ma_event e

    ma_uint32 counter
} ma_fence

ma_result ma_fence_init(ma_fence* pFence)
void ma_fence_uninit(ma_fence* pFence)
ma_result ma_fence_acquire(ma_fence* pFence)
ma_result ma_fence_release(ma_fence* pFence)
ma_result ma_fence_wait(ma_fence* pFence)






typedef void ma_async_notification

typedef struct
{
    void (* onSignal)(ma_async_notification* pNotification)
} ma_async_notification_callbacks

ma_result ma_async_notification_signal(ma_async_notification* pNotification)







typedef struct
{
    ma_async_notification_callbacks cb
    ma_bool32 signalled
} ma_async_notification_poll

ma_result ma_async_notification_poll_init(ma_async_notification_poll* pNotificationPoll)
ma_bool32 ma_async_notification_poll_is_signalled(const ma_async_notification_poll* pNotificationPoll)







typedef struct
{
    ma_async_notification_callbacks cb

    ma_event e

} ma_async_notification_event

ma_result ma_async_notification_event_init(ma_async_notification_event* pNotificationEvent)
ma_result ma_async_notification_event_uninit(ma_async_notification_event* pNotificationEvent)
ma_result ma_async_notification_event_wait(ma_async_notification_event* pNotificationEvent)
ma_result ma_async_notification_event_signal(ma_async_notification_event* pNotificationEvent)

typedef struct
{
    ma_uint32 capacity
} ma_slot_allocator_config

ma_slot_allocator_config ma_slot_allocator_config_init(ma_uint32 capacity)


typedef struct
{
    _Alignas(4) ma_uint32 bitfield
} ma_slot_allocator_group

typedef struct
{
    ma_slot_allocator_group* pGroups
    ma_uint32* pSlots
    ma_uint32 count
    ma_uint32 capacity


    ma_bool32 _ownsHeap
    void* _pHeap
} ma_slot_allocator

ma_result ma_slot_allocator_get_heap_size(const ma_slot_allocator_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_slot_allocator_init_preallocated(const ma_slot_allocator_config* pConfig, void* pHeap, ma_slot_allocator* pAllocator)
ma_result ma_slot_allocator_init(const ma_slot_allocator_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_slot_allocator* pAllocator)
void ma_slot_allocator_uninit(ma_slot_allocator* pAllocator, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_slot_allocator_alloc(ma_slot_allocator* pAllocator, ma_uint64* pSlot)
ma_result ma_slot_allocator_free(ma_slot_allocator* pAllocator, ma_uint64 slot)


typedef struct ma_job ma_job





typedef ma_result (* ma_job_proc)(ma_job* pJob)


typedef enum
{

    MA_JOB_TYPE_QUIT = 0,
    MA_JOB_TYPE_CUSTOM,


    MA_JOB_TYPE_RESOURCE_MANAGER_LOAD_DATA_BUFFER_NODE,
    MA_JOB_TYPE_RESOURCE_MANAGER_FREE_DATA_BUFFER_NODE,
    MA_JOB_TYPE_RESOURCE_MANAGER_PAGE_DATA_BUFFER_NODE,
    MA_JOB_TYPE_RESOURCE_MANAGER_LOAD_DATA_BUFFER,
    MA_JOB_TYPE_RESOURCE_MANAGER_FREE_DATA_BUFFER,
    MA_JOB_TYPE_RESOURCE_MANAGER_LOAD_DATA_STREAM,
    MA_JOB_TYPE_RESOURCE_MANAGER_FREE_DATA_STREAM,
    MA_JOB_TYPE_RESOURCE_MANAGER_PAGE_DATA_STREAM,
    MA_JOB_TYPE_RESOURCE_MANAGER_SEEK_DATA_STREAM,


    MA_JOB_TYPE_DEVICE_AAUDIO_REROUTE,


    MA_JOB_TYPE_COUNT
} ma_job_type

struct ma_job
{
    union
    {
        struct
        {
            ma_uint16 code
            ma_uint16 slot
            ma_uint32 refcount
        } breakup
        ma_uint64 allocation
    } toc
    _Alignas(8) ma_uint64 next
    ma_uint32 order

    union
    {

        struct
        {
            ma_job_proc proc
            ma_uintptr data0
            ma_uintptr data1
        } custom


        union
        {
            struct
            {
                                         void* pResourceManager
                                                          void* pDataBufferNode
                char* pFilePath
                wchar_t* pFilePathW
                ma_uint32 flags
                ma_async_notification* pInitNotification
                ma_async_notification* pDoneNotification
                ma_fence* pInitFence
                ma_fence* pDoneFence
            } loadDataBufferNode
            struct
            {
                                         void* pResourceManager
                                                          void* pDataBufferNode
                ma_async_notification* pDoneNotification
                ma_fence* pDoneFence
            } freeDataBufferNode
            struct
            {
                                         void* pResourceManager
                                                          void* pDataBufferNode
                                void* pDecoder
                ma_async_notification* pDoneNotification
                ma_fence* pDoneFence
            } pageDataBufferNode

            struct
            {
                                                     void* pDataBuffer
                ma_async_notification* pInitNotification
                ma_async_notification* pDoneNotification
                ma_fence* pInitFence
                ma_fence* pDoneFence
                ma_uint64 rangeBegInPCMFrames
                ma_uint64 rangeEndInPCMFrames
                ma_uint64 loopPointBegInPCMFrames
                ma_uint64 loopPointEndInPCMFrames
                ma_uint32 isLooping
            } loadDataBuffer
            struct
            {
                                                     void* pDataBuffer
                ma_async_notification* pDoneNotification
                ma_fence* pDoneFence
            } freeDataBuffer

            struct
            {
                                                     void* pDataStream
                char* pFilePath
                wchar_t* pFilePathW
                ma_uint64 initialSeekPoint
                ma_async_notification* pInitNotification
                ma_fence* pInitFence
            } loadDataStream
            struct
            {
                                                     void* pDataStream
                ma_async_notification* pDoneNotification
                ma_fence* pDoneFence
            } freeDataStream
            struct
            {
                                                     void* pDataStream
                ma_uint32 pageIndex
            } pageDataStream
            struct
            {
                                                     void* pDataStream
                ma_uint64 frameIndex
            } seekDataStream
        } resourceManager


        union
        {
            union
            {
                struct
                {
                                   void* pDevice
                                       ma_uint32 deviceType
                } reroute
            } aaudio
        } device
    } data
}

ma_job ma_job_init(ma_uint16 code)
ma_result ma_job_process(ma_job* pJob)

typedef enum
{
    MA_JOB_QUEUE_FLAG_NON_BLOCKING = 0x00000001
} ma_job_queue_flags

typedef struct
{
    ma_uint32 flags
    ma_uint32 capacity
} ma_job_queue_config

ma_job_queue_config ma_job_queue_config_init(ma_uint32 flags, ma_uint32 capacity)


typedef struct
{
    ma_uint32 flags
    ma_uint32 capacity
    _Alignas(8) ma_uint64 head
    _Alignas(8) ma_uint64 tail

    ma_semaphore sem

    ma_slot_allocator allocator
    ma_job* pJobs

    ma_spinlock lock



    void* _pHeap
    ma_bool32 _ownsHeap
} ma_job_queue

ma_result ma_job_queue_get_heap_size(const ma_job_queue_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_job_queue_init_preallocated(const ma_job_queue_config* pConfig, void* pHeap, ma_job_queue* pQueue)
ma_result ma_job_queue_init(const ma_job_queue_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_job_queue* pQueue)
void ma_job_queue_uninit(ma_job_queue* pQueue, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_job_queue_post(ma_job_queue* pQueue, const ma_job* pJob)
ma_result ma_job_queue_next(ma_job_queue* pQueue, ma_job* pJob)

typedef enum
{
    ma_device_state_uninitialized = 0,
    ma_device_state_stopped = 1,
    ma_device_state_started = 2,
    ma_device_state_starting = 3,
    ma_device_state_stopping = 4
} ma_device_state

typedef struct { _Alignas(4) ma_device_state value } ma_atomic_device_state

typedef enum
{
    ma_backend_wasapi,
    ma_backend_dsound,
    ma_backend_winmm,
    ma_backend_coreaudio,
    ma_backend_sndio,
    ma_backend_audio4,
    ma_backend_oss,
    ma_backend_pulseaudio,
    ma_backend_alsa,
    ma_backend_jack,
    ma_backend_aaudio,
    ma_backend_opensl,
    ma_backend_webaudio,
    ma_backend_custom,
    ma_backend_null
} ma_backend

typedef struct
{
    ma_bool32 noThread
    ma_uint32 jobQueueCapacity
    ma_uint32 jobQueueFlags
} ma_device_job_thread_config

ma_device_job_thread_config ma_device_job_thread_config_init(void)

typedef struct
{
    ma_thread thread
    ma_job_queue jobQueue
    ma_bool32 _hasThread
} ma_device_job_thread

ma_result ma_device_job_thread_init(const ma_device_job_thread_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_device_job_thread* pJobThread)
void ma_device_job_thread_uninit(ma_device_job_thread* pJobThread, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_device_job_thread_post(ma_device_job_thread* pJobThread, const ma_job* pJob)
ma_result ma_device_job_thread_next(ma_device_job_thread* pJobThread, ma_job* pJob)




typedef enum
{
    ma_device_notification_type_started,
    ma_device_notification_type_stopped,
    ma_device_notification_type_rerouted,
    ma_device_notification_type_interruption_began,
    ma_device_notification_type_interruption_ended,
    ma_device_notification_type_unlocked
} ma_device_notification_type

typedef struct
{
    ma_device* pDevice
    ma_device_notification_type type
    union
    {
        struct
        {
            int _unused
        } started
        struct
        {
            int _unused
        } stopped
        struct
        {
            int _unused
        } rerouted
        struct
        {
            int _unused
        } interruption
    } data
} ma_device_notification

typedef void (* ma_device_notification_proc)(const ma_device_notification* pNotification)

typedef void (* ma_device_data_proc)(ma_device* pDevice, void* pOutput, const void* pInput, ma_uint32 frameCount)

typedef void (* ma_stop_proc)(ma_device* pDevice)

typedef enum
{
    ma_device_type_playback = 1,
    ma_device_type_capture = 2,
    ma_device_type_duplex = ma_device_type_playback | ma_device_type_capture,
    ma_device_type_loopback = 4
} ma_device_type

typedef enum
{
    ma_share_mode_shared = 0,
    ma_share_mode_exclusive
} ma_share_mode


typedef enum
{
    ma_ios_session_category_default = 0,
    ma_ios_session_category_none,
    ma_ios_session_category_ambient,
    ma_ios_session_category_solo_ambient,
    ma_ios_session_category_playback,
    ma_ios_session_category_record,
    ma_ios_session_category_play_and_record,
    ma_ios_session_category_multi_route
} ma_ios_session_category


typedef enum
{
    ma_ios_session_category_option_mix_with_others = 0x01,
    ma_ios_session_category_option_duck_others = 0x02,
    ma_ios_session_category_option_allow_bluetooth = 0x04,
    ma_ios_session_category_option_default_to_speaker = 0x08,
    ma_ios_session_category_option_interrupt_spoken_audio_and_mix_with_others = 0x11,
    ma_ios_session_category_option_allow_bluetooth_a2dp = 0x20,
    ma_ios_session_category_option_allow_air_play = 0x40,
} ma_ios_session_category_option


typedef enum
{
    ma_opensl_stream_type_default = 0,
    ma_opensl_stream_type_voice,
    ma_opensl_stream_type_system,
    ma_opensl_stream_type_ring,
    ma_opensl_stream_type_media,
    ma_opensl_stream_type_alarm,
    ma_opensl_stream_type_notification
} ma_opensl_stream_type


typedef enum
{
    ma_opensl_recording_preset_default = 0,
    ma_opensl_recording_preset_generic,
    ma_opensl_recording_preset_camcorder,
    ma_opensl_recording_preset_voice_recognition,
    ma_opensl_recording_preset_voice_communication,
    ma_opensl_recording_preset_voice_unprocessed
} ma_opensl_recording_preset


typedef enum
{
    ma_wasapi_usage_default = 0,
    ma_wasapi_usage_games,
    ma_wasapi_usage_pro_audio,
} ma_wasapi_usage


typedef enum
{
    ma_aaudio_usage_default = 0,
    ma_aaudio_usage_media,
    ma_aaudio_usage_voice_communication,
    ma_aaudio_usage_voice_communication_signalling,
    ma_aaudio_usage_alarm,
    ma_aaudio_usage_notification,
    ma_aaudio_usage_notification_ringtone,
    ma_aaudio_usage_notification_event,
    ma_aaudio_usage_assistance_accessibility,
    ma_aaudio_usage_assistance_navigation_guidance,
    ma_aaudio_usage_assistance_sonification,
    ma_aaudio_usage_game,
    ma_aaudio_usage_assitant,
    ma_aaudio_usage_emergency,
    ma_aaudio_usage_safety,
    ma_aaudio_usage_vehicle_status,
    ma_aaudio_usage_announcement
} ma_aaudio_usage


typedef enum
{
    ma_aaudio_content_type_default = 0,
    ma_aaudio_content_type_speech,
    ma_aaudio_content_type_music,
    ma_aaudio_content_type_movie,
    ma_aaudio_content_type_sonification
} ma_aaudio_content_type


typedef enum
{
    ma_aaudio_input_preset_default = 0,
    ma_aaudio_input_preset_generic,
    ma_aaudio_input_preset_camcorder,
    ma_aaudio_input_preset_voice_recognition,
    ma_aaudio_input_preset_voice_communication,
    ma_aaudio_input_preset_unprocessed,
    ma_aaudio_input_preset_voice_performance
} ma_aaudio_input_preset

typedef enum
{
    ma_aaudio_allow_capture_default = 0,
    ma_aaudio_allow_capture_by_all,
    ma_aaudio_allow_capture_by_system,
    ma_aaudio_allow_capture_by_none
} ma_aaudio_allowed_capture_policy

typedef union
{
    ma_int64 counter
    double counterD
} ma_timer

typedef union
{
    ma_wchar_win32 wasapi[64]
    ma_uint8 dsound[16]
                 ma_uint32 winmm
    char alsa[256]
    char pulse[256]
    int jack
    char coreaudio[256]
    char sndio[256]
    char audio4[256]
    char oss[64]
    ma_int32 aaudio
    ma_uint32 opensl
    char webaudio[32]
    union
    {
        int i
        char s[256]
        void* p
    } custom
    int nullbackend
} ma_device_id


typedef struct ma_context_config ma_context_config
typedef struct ma_device_config ma_device_config
typedef struct ma_backend_callbacks ma_backend_callbacks







typedef struct
{

    ma_device_id id
    char name[255 + 1]
    ma_bool32 isDefault

    ma_uint32 nativeDataFormatCount
    struct
    {
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        ma_uint32 flags
    } nativeDataFormats[ 64]
} ma_device_info

struct ma_device_config
{
    ma_device_type deviceType
    ma_uint32 sampleRate
    ma_uint32 periodSizeInFrames
    ma_uint32 periodSizeInMilliseconds
    ma_uint32 periods
    ma_performance_profile performanceProfile
    ma_bool8 noPreSilencedOutputBuffer
    ma_bool8 noClip
    ma_bool8 noDisableDenormals
    ma_bool8 noFixedSizedCallback
    ma_device_data_proc dataCallback
    ma_device_notification_proc notificationCallback
    ma_stop_proc stopCallback
    void* pUserData
    ma_resampler_config resampling
    struct
    {
        const ma_device_id* pDeviceID
        ma_format format
        ma_uint32 channels
        ma_channel* pChannelMap
        ma_channel_mix_mode channelMixMode
        ma_bool32 calculateLFEFromSpatialChannels
        ma_share_mode shareMode
    } playback
    struct
    {
        const ma_device_id* pDeviceID
        ma_format format
        ma_uint32 channels
        ma_channel* pChannelMap
        ma_channel_mix_mode channelMixMode
        ma_bool32 calculateLFEFromSpatialChannels
        ma_share_mode shareMode
    } capture

    struct
    {
        ma_wasapi_usage usage
        ma_bool8 noAutoConvertSRC
        ma_bool8 noDefaultQualitySRC
        ma_bool8 noAutoStreamRouting
        ma_bool8 noHardwareOffloading
        ma_uint32 loopbackProcessID
        ma_bool8 loopbackProcessExclude
    } wasapi
    struct
    {
        ma_bool32 noMMap
        ma_bool32 noAutoFormat
        ma_bool32 noAutoChannels
        ma_bool32 noAutoResample
    } alsa
    struct
    {
        const char* pStreamNamePlayback
        const char* pStreamNameCapture
    } pulse
    struct
    {
        ma_bool32 allowNominalSampleRateChange
    } coreaudio
    struct
    {
        ma_opensl_stream_type streamType
        ma_opensl_recording_preset recordingPreset
        ma_bool32 enableCompatibilityWorkarounds
    } opensl
    struct
    {
        ma_aaudio_usage usage
        ma_aaudio_content_type contentType
        ma_aaudio_input_preset inputPreset
        ma_aaudio_allowed_capture_policy allowedCapturePolicy
        ma_bool32 noAutoStartAfterReroute
        ma_bool32 enableCompatibilityWorkarounds
    } aaudio
}

typedef ma_bool32 (* ma_enum_devices_callback_proc)(ma_context* pContext, ma_device_type deviceType, const ma_device_info* pInfo, void* pUserData)





typedef struct
{
    const ma_device_id* pDeviceID
    ma_share_mode shareMode
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_channel channelMap[254]
    ma_uint32 periodSizeInFrames
    ma_uint32 periodSizeInMilliseconds
    ma_uint32 periodCount
} ma_device_descriptor

struct ma_backend_callbacks
{
    ma_result (* onContextInit)(ma_context* pContext, const ma_context_config* pConfig, ma_backend_callbacks* pCallbacks)
    ma_result (* onContextUninit)(ma_context* pContext)
    ma_result (* onContextEnumerateDevices)(ma_context* pContext, ma_enum_devices_callback_proc callback, void* pUserData)
    ma_result (* onContextGetDeviceInfo)(ma_context* pContext, ma_device_type deviceType, const ma_device_id* pDeviceID, ma_device_info* pDeviceInfo)
    ma_result (* onDeviceInit)(ma_device* pDevice, const ma_device_config* pConfig, ma_device_descriptor* pDescriptorPlayback, ma_device_descriptor* pDescriptorCapture)
    ma_result (* onDeviceUninit)(ma_device* pDevice)
    ma_result (* onDeviceStart)(ma_device* pDevice)
    ma_result (* onDeviceStop)(ma_device* pDevice)
    ma_result (* onDeviceRead)(ma_device* pDevice, void* pFrames, ma_uint32 frameCount, ma_uint32* pFramesRead)
    ma_result (* onDeviceWrite)(ma_device* pDevice, const void* pFrames, ma_uint32 frameCount, ma_uint32* pFramesWritten)
    ma_result (* onDeviceDataLoop)(ma_device* pDevice)
    ma_result (* onDeviceDataLoopWakeup)(ma_device* pDevice)
    ma_result (* onDeviceGetInfo)(ma_device* pDevice, ma_device_type type, ma_device_info* pDeviceInfo)
}

struct ma_context_config
{
    ma_log* pLog
    ma_thread_priority threadPriority
    size_t threadStackSize
    void* pUserData
    ma_allocation_callbacks allocationCallbacks
    struct
    {
        ma_bool32 useVerboseDeviceEnumeration
    } alsa
    struct
    {
        const char* pApplicationName
        const char* pServerName
        ma_bool32 tryAutoSpawn
    } pulse
    struct
    {
        ma_ios_session_category sessionCategory
        ma_uint32 sessionCategoryOptions
        ma_bool32 noAudioSessionActivate
        ma_bool32 noAudioSessionDeactivate
    } coreaudio
    struct
    {
        const char* pClientName
        ma_bool32 tryStartServer
    } jack
    ma_backend_callbacks custom
}


typedef struct
{
    int code
    ma_event* pEvent
    union
    {
        struct
        {
            int _unused
        } quit
        struct
        {
            ma_device_type deviceType
            void* pAudioClient
            void** ppAudioClientService
            ma_result* pResult
        } createAudioClient
        struct
        {
            ma_device* pDevice
            ma_device_type deviceType
        } releaseAudioClient
    } data
} ma_context_command__wasapi

struct ma_context
{
    ma_backend_callbacks callbacks
    ma_backend backend
    ma_log* pLog
    ma_log log
    ma_thread_priority threadPriority
    size_t threadStackSize
    void* pUserData
    ma_allocation_callbacks allocationCallbacks
    ma_mutex deviceEnumLock
    ma_mutex deviceInfoLock
    ma_uint32 deviceInfoCapacity
    ma_uint32 playbackDeviceInfoCount
    ma_uint32 captureDeviceInfoCount
    ma_device_info* pDeviceInfos

    union
    {

        struct
        {
            ma_handle hCoreFoundation
            ma_proc CFStringGetCString
            ma_proc CFRelease

            ma_handle hCoreAudio
            ma_proc AudioObjectGetPropertyData
            ma_proc AudioObjectGetPropertyDataSize
            ma_proc AudioObjectSetPropertyData
            ma_proc AudioObjectAddPropertyListener
            ma_proc AudioObjectRemovePropertyListener

            ma_handle hAudioUnit
            ma_proc AudioComponentFindNext
            ma_proc AudioComponentInstanceDispose
            ma_proc AudioComponentInstanceNew
            ma_proc AudioOutputUnitStart
            ma_proc AudioOutputUnitStop
            ma_proc AudioUnitAddPropertyListener
            ma_proc AudioUnitGetPropertyInfo
            ma_proc AudioUnitGetProperty
            ma_proc AudioUnitSetProperty
            ma_proc AudioUnitInitialize
            ma_proc AudioUnitRender

                               ma_ptr component
            ma_bool32 noAudioSessionDeactivate
        } coreaudio

        struct
        {
            int _unused
        } null_backend

    }

    union
    {

        struct
        {
            int _unused
        } posix

        int _unused
    }
}

struct ma_device
{
    ma_context* pContext
    ma_device_type type
    ma_uint32 sampleRate
    ma_atomic_device_state state
    ma_device_data_proc onData
    ma_device_notification_proc onNotification
    ma_stop_proc onStop
    void* pUserData
    ma_mutex startStopLock
    ma_event wakeupEvent
    ma_event startEvent
    ma_event stopEvent
    ma_thread thread
    ma_result workResult
    ma_bool8 isOwnerOfContext
    ma_bool8 noPreSilencedOutputBuffer
    ma_bool8 noClip
    ma_bool8 noDisableDenormals
    ma_bool8 noFixedSizedCallback
    ma_atomic_float masterVolumeFactor
    ma_duplex_rb duplexRB
    struct
    {
        ma_resample_algorithm algorithm
        ma_resampling_backend_vtable* pBackendVTable
        void* pBackendUserData
        struct
        {
            ma_uint32 lpfOrder
        } linear
    } resampling
    struct
    {
        ma_device_id* pID
        ma_device_id id
        char name[255 + 1]
        ma_share_mode shareMode
        ma_format format
        ma_uint32 channels
        ma_channel channelMap[254]
        ma_format internalFormat
        ma_uint32 internalChannels
        ma_uint32 internalSampleRate
        ma_channel internalChannelMap[254]
        ma_uint32 internalPeriodSizeInFrames
        ma_uint32 internalPeriods
        ma_channel_mix_mode channelMixMode
        ma_bool32 calculateLFEFromSpatialChannels
        ma_data_converter converter
        void* pIntermediaryBuffer
        ma_uint32 intermediaryBufferCap
        ma_uint32 intermediaryBufferLen
        void* pInputCache
        ma_uint64 inputCacheCap
        ma_uint64 inputCacheConsumed
        ma_uint64 inputCacheRemaining
    } playback
    struct
    {
        ma_device_id* pID
        ma_device_id id
        char name[255 + 1]
        ma_share_mode shareMode
        ma_format format
        ma_uint32 channels
        ma_channel channelMap[254]
        ma_format internalFormat
        ma_uint32 internalChannels
        ma_uint32 internalSampleRate
        ma_channel internalChannelMap[254]
        ma_uint32 internalPeriodSizeInFrames
        ma_uint32 internalPeriods
        ma_channel_mix_mode channelMixMode
        ma_bool32 calculateLFEFromSpatialChannels
        ma_data_converter converter
        void* pIntermediaryBuffer
        ma_uint32 intermediaryBufferCap
        ma_uint32 intermediaryBufferLen
    } capture

    union
    {

        struct
        {
            ma_uint32 deviceObjectIDPlayback
            ma_uint32 deviceObjectIDCapture
                          ma_ptr audioUnitPlayback
                          ma_ptr audioUnitCapture
                                 ma_ptr pAudioBufferList
            ma_uint32 audioBufferCapInFrames
            ma_event stopEvent
            ma_uint32 originalPeriodSizeInFrames
            ma_uint32 originalPeriodSizeInMilliseconds
            ma_uint32 originalPeriods
            ma_performance_profile originalPerformanceProfile
            ma_bool32 isDefaultPlaybackDevice
            ma_bool32 isDefaultCaptureDevice
            ma_bool32 isSwitchingPlaybackDevice
            ma_bool32 isSwitchingCaptureDevice
            void* pNotificationHandler
        } coreaudio

        struct
        {
            ma_thread deviceThread
            ma_event operationEvent
            ma_event operationCompletionEvent
            ma_semaphore operationSemaphore
            ma_uint32 operation
            ma_result operationResult
            ma_timer timer
            double priorRunTime
            ma_uint32 currentPeriodFramesRemainingPlayback
            ma_uint32 currentPeriodFramesRemainingCapture
            ma_uint64 lastProcessedFramePlayback
            ma_uint64 lastProcessedFrameCapture
            ma_atomic_bool32 isStarted
        } null_device

    }
}



#pragma GCC diagnostic pop

ma_context_config ma_context_config_init(void)

ma_result ma_context_init(const ma_backend backends[], ma_uint32 backendCount, const ma_context_config* pConfig, ma_context* pContext)

ma_result ma_context_uninit(ma_context* pContext)






size_t ma_context_sizeof(void)

ma_log* ma_context_get_log(ma_context* pContext)

ma_result ma_context_enumerate_devices(ma_context* pContext, ma_enum_devices_callback_proc callback, void* pUserData)

ma_result ma_context_get_devices(ma_context* pContext, ma_device_info** ppPlaybackDeviceInfos, ma_uint32* pPlaybackDeviceCount, ma_device_info** ppCaptureDeviceInfos, ma_uint32* pCaptureDeviceCount)

ma_result ma_context_get_device_info(ma_context* pContext, ma_device_type deviceType, const ma_device_id* pDeviceID, ma_device_info* pDeviceInfo)

ma_bool32 ma_context_is_loopback_supported(ma_context* pContext)

ma_device_config ma_device_config_init(ma_device_type deviceType)

ma_result ma_device_init(ma_context* pContext, const ma_device_config* pConfig, ma_device* pDevice)

ma_result ma_device_init_ex(const ma_backend backends[], ma_uint32 backendCount, const ma_context_config* pContextConfig, const ma_device_config* pConfig, ma_device* pDevice)

void ma_device_uninit(ma_device* pDevice)





ma_context* ma_device_get_context(ma_device* pDevice)




ma_log* ma_device_get_log(ma_device* pDevice)

ma_result ma_device_get_info(ma_device* pDevice, ma_device_type type, ma_device_info* pDeviceInfo)
ma_result ma_device_get_name(ma_device* pDevice, ma_device_type type, char* pName, size_t nameCap, size_t* pLengthNotIncludingNullTerminator)
ma_result ma_device_start(ma_device* pDevice)
ma_result ma_device_stop(ma_device* pDevice)
ma_bool32 ma_device_is_started(const ma_device* pDevice)
ma_device_state ma_device_get_state(const ma_device* pDevice)
ma_result ma_device_post_init(ma_device* pDevice, ma_device_type deviceType, const ma_device_descriptor* pPlaybackDescriptor, const ma_device_descriptor* pCaptureDescriptor)
ma_result ma_device_set_master_volume(ma_device* pDevice, float volume)
ma_result ma_device_get_master_volume(ma_device* pDevice, float* pVolume)
ma_result ma_device_set_master_volume_db(ma_device* pDevice, float gainDB)
ma_result ma_device_get_master_volume_db(ma_device* pDevice, float* pGainDB)
ma_result ma_device_handle_backend_data_callback(ma_device* pDevice, void* pOutput, const void* pInput, ma_uint32 frameCount)
ma_uint32 ma_calculate_buffer_size_in_frames_from_descriptor(const ma_device_descriptor* pDescriptor, ma_uint32 nativeSampleRate, ma_performance_profile performanceProfile)






const char* ma_get_backend_name(ma_backend backend)




ma_result ma_get_backend_from_name(const char* pBackendName, ma_backend* pBackend)




ma_bool32 ma_is_backend_enabled(ma_backend backend)

ma_result ma_get_enabled_backends(ma_backend* pBackends, size_t backendCap, size_t* pBackendCount)




ma_bool32 ma_is_loopback_supported(ma_backend backend)
ma_uint32 ma_calculate_buffer_size_in_milliseconds_from_frames(ma_uint32 bufferSizeInFrames, ma_uint32 sampleRate)




ma_uint32 ma_calculate_buffer_size_in_frames_from_milliseconds(ma_uint32 bufferSizeInMilliseconds, ma_uint32 sampleRate)




void ma_copy_pcm_frames(void* dst, const void* src, ma_uint64 frameCount, ma_format format, ma_uint32 channels)
void ma_silence_pcm_frames(void* p, ma_uint64 frameCount, ma_format format, ma_uint32 channels)





void* ma_offset_pcm_frames_ptr(void* p, ma_uint64 offsetInFrames, ma_format format, ma_uint32 channels)
const void* ma_offset_pcm_frames_const_ptr(const void* p, ma_uint64 offsetInFrames, ma_format format, ma_uint32 channels)
static inline __attribute__((always_inline)) float* ma_offset_pcm_frames_ptr_f32(float* p, ma_uint64 offsetInFrames, ma_uint32 channels) { return (float*)ma_offset_pcm_frames_ptr((void*)p, offsetInFrames, ma_format_f32, channels) }
static inline __attribute__((always_inline)) const float* ma_offset_pcm_frames_const_ptr_f32(const float* p, ma_uint64 offsetInFrames, ma_uint32 channels) { return (const float*)ma_offset_pcm_frames_const_ptr((const void*)p, offsetInFrames, ma_format_f32, channels) }





void ma_clip_samples_u8(ma_uint8* pDst, const ma_int16* pSrc, ma_uint64 count)
void ma_clip_samples_s16(ma_int16* pDst, const ma_int32* pSrc, ma_uint64 count)
void ma_clip_samples_s24(ma_uint8* pDst, const ma_int64* pSrc, ma_uint64 count)
void ma_clip_samples_s32(ma_int32* pDst, const ma_int64* pSrc, ma_uint64 count)
void ma_clip_samples_f32(float* pDst, const float* pSrc, ma_uint64 count)
void ma_clip_pcm_frames(void* pDst, const void* pSrc, ma_uint64 frameCount, ma_format format, ma_uint32 channels)






void ma_copy_and_apply_volume_factor_u8(ma_uint8* pSamplesOut, const ma_uint8* pSamplesIn, ma_uint64 sampleCount, float factor)
void ma_copy_and_apply_volume_factor_s16(ma_int16* pSamplesOut, const ma_int16* pSamplesIn, ma_uint64 sampleCount, float factor)
void ma_copy_and_apply_volume_factor_s24(void* pSamplesOut, const void* pSamplesIn, ma_uint64 sampleCount, float factor)
void ma_copy_and_apply_volume_factor_s32(ma_int32* pSamplesOut, const ma_int32* pSamplesIn, ma_uint64 sampleCount, float factor)
void ma_copy_and_apply_volume_factor_f32(float* pSamplesOut, const float* pSamplesIn, ma_uint64 sampleCount, float factor)

void ma_apply_volume_factor_u8(ma_uint8* pSamples, ma_uint64 sampleCount, float factor)
void ma_apply_volume_factor_s16(ma_int16* pSamples, ma_uint64 sampleCount, float factor)
void ma_apply_volume_factor_s24(void* pSamples, ma_uint64 sampleCount, float factor)
void ma_apply_volume_factor_s32(ma_int32* pSamples, ma_uint64 sampleCount, float factor)
void ma_apply_volume_factor_f32(float* pSamples, ma_uint64 sampleCount, float factor)

void ma_copy_and_apply_volume_factor_pcm_frames_u8(ma_uint8* pFramesOut, const ma_uint8* pFramesIn, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames_s16(ma_int16* pFramesOut, const ma_int16* pFramesIn, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames_s24(void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames_s32(ma_int32* pFramesOut, const ma_int32* pFramesIn, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames_f32(float* pFramesOut, const float* pFramesIn, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames(void* pFramesOut, const void* pFramesIn, ma_uint64 frameCount, ma_format format, ma_uint32 channels, float factor)

void ma_apply_volume_factor_pcm_frames_u8(ma_uint8* pFrames, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_apply_volume_factor_pcm_frames_s16(ma_int16* pFrames, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_apply_volume_factor_pcm_frames_s24(void* pFrames, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_apply_volume_factor_pcm_frames_s32(ma_int32* pFrames, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_apply_volume_factor_pcm_frames_f32(float* pFrames, ma_uint64 frameCount, ma_uint32 channels, float factor)
void ma_apply_volume_factor_pcm_frames(void* pFrames, ma_uint64 frameCount, ma_format format, ma_uint32 channels, float factor)

void ma_copy_and_apply_volume_factor_per_channel_f32(float* pFramesOut, const float* pFramesIn, ma_uint64 frameCount, ma_uint32 channels, float* pChannelGains)


void ma_copy_and_apply_volume_and_clip_samples_u8(ma_uint8* pDst, const ma_int16* pSrc, ma_uint64 count, float volume)
void ma_copy_and_apply_volume_and_clip_samples_s16(ma_int16* pDst, const ma_int32* pSrc, ma_uint64 count, float volume)
void ma_copy_and_apply_volume_and_clip_samples_s24(ma_uint8* pDst, const ma_int64* pSrc, ma_uint64 count, float volume)
void ma_copy_and_apply_volume_and_clip_samples_s32(ma_int32* pDst, const ma_int64* pSrc, ma_uint64 count, float volume)
void ma_copy_and_apply_volume_and_clip_samples_f32(float* pDst, const float* pSrc, ma_uint64 count, float volume)
void ma_copy_and_apply_volume_and_clip_pcm_frames(void* pDst, const void* pSrc, ma_uint64 frameCount, ma_format format, ma_uint32 channels, float volume)





float ma_volume_linear_to_db(float factor)




float ma_volume_db_to_linear(float gain)







ma_result ma_mix_pcm_frames_f32(float* pDst, const float* pSrc, ma_uint64 frameCount, ma_uint32 channels, float volume)

typedef void ma_vfs
typedef ma_handle ma_vfs_file

typedef enum
{
    MA_OPEN_MODE_READ = 0x00000001,
    MA_OPEN_MODE_WRITE = 0x00000002
} ma_open_mode_flags

typedef enum
{
    ma_seek_origin_start,
    ma_seek_origin_current,
    ma_seek_origin_end
} ma_seek_origin

typedef struct
{
    ma_uint64 sizeInBytes
} ma_file_info

typedef struct
{
    ma_result (* onOpen) (ma_vfs* pVFS, const char* pFilePath, ma_uint32 openMode, ma_vfs_file* pFile)
    ma_result (* onOpenW)(ma_vfs* pVFS, const wchar_t* pFilePath, ma_uint32 openMode, ma_vfs_file* pFile)
    ma_result (* onClose)(ma_vfs* pVFS, ma_vfs_file file)
    ma_result (* onRead) (ma_vfs* pVFS, ma_vfs_file file, void* pDst, size_t sizeInBytes, size_t* pBytesRead)
    ma_result (* onWrite)(ma_vfs* pVFS, ma_vfs_file file, const void* pSrc, size_t sizeInBytes, size_t* pBytesWritten)
    ma_result (* onSeek) (ma_vfs* pVFS, ma_vfs_file file, ma_int64 offset, ma_seek_origin origin)
    ma_result (* onTell) (ma_vfs* pVFS, ma_vfs_file file, ma_int64* pCursor)
    ma_result (* onInfo) (ma_vfs* pVFS, ma_vfs_file file, ma_file_info* pInfo)
} ma_vfs_callbacks

ma_result ma_vfs_open(ma_vfs* pVFS, const char* pFilePath, ma_uint32 openMode, ma_vfs_file* pFile)
ma_result ma_vfs_open_w(ma_vfs* pVFS, const wchar_t* pFilePath, ma_uint32 openMode, ma_vfs_file* pFile)
ma_result ma_vfs_close(ma_vfs* pVFS, ma_vfs_file file)
ma_result ma_vfs_read(ma_vfs* pVFS, ma_vfs_file file, void* pDst, size_t sizeInBytes, size_t* pBytesRead)
ma_result ma_vfs_write(ma_vfs* pVFS, ma_vfs_file file, const void* pSrc, size_t sizeInBytes, size_t* pBytesWritten)
ma_result ma_vfs_seek(ma_vfs* pVFS, ma_vfs_file file, ma_int64 offset, ma_seek_origin origin)
ma_result ma_vfs_tell(ma_vfs* pVFS, ma_vfs_file file, ma_int64* pCursor)
ma_result ma_vfs_info(ma_vfs* pVFS, ma_vfs_file file, ma_file_info* pInfo)
ma_result ma_vfs_open_and_read_file(ma_vfs* pVFS, const char* pFilePath, void** ppData, size_t* pSize, const ma_allocation_callbacks* pAllocationCallbacks)

typedef struct
{
    ma_vfs_callbacks cb
    ma_allocation_callbacks allocationCallbacks
} ma_default_vfs

ma_result ma_default_vfs_init(ma_default_vfs* pVFS, const ma_allocation_callbacks* pAllocationCallbacks)



typedef ma_result (* ma_read_proc)(void* pUserData, void* pBufferOut, size_t bytesToRead, size_t* pBytesRead)
typedef ma_result (* ma_seek_proc)(void* pUserData, ma_int64 offset, ma_seek_origin origin)
typedef ma_result (* ma_tell_proc)(void* pUserData, ma_int64* pCursor)




typedef enum
{
    ma_encoding_format_unknown = 0,
    ma_encoding_format_wav,
    ma_encoding_format_flac,
    ma_encoding_format_mp3,
    ma_encoding_format_vorbis
} ma_encoding_format

typedef struct ma_decoder ma_decoder


typedef struct
{
    ma_format preferredFormat
    ma_uint32 seekPointCount
} ma_decoding_backend_config

ma_decoding_backend_config ma_decoding_backend_config_init(ma_format preferredFormat, ma_uint32 seekPointCount)


typedef struct
{
    ma_result (* onInit )(void* pUserData, ma_read_proc onRead, ma_seek_proc onSeek, ma_tell_proc onTell, void* pReadSeekTellUserData, const ma_decoding_backend_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_data_source** ppBackend)
    ma_result (* onInitFile )(void* pUserData, const char* pFilePath, const ma_decoding_backend_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_data_source** ppBackend)
    ma_result (* onInitFileW )(void* pUserData, const wchar_t* pFilePath, const ma_decoding_backend_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_data_source** ppBackend)
    ma_result (* onInitMemory)(void* pUserData, const void* pData, size_t dataSize, const ma_decoding_backend_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_data_source** ppBackend)
    void (* onUninit )(void* pUserData, ma_data_source* pBackend, const ma_allocation_callbacks* pAllocationCallbacks)
} ma_decoding_backend_vtable


typedef ma_result (* ma_decoder_read_proc)(ma_decoder* pDecoder, void* pBufferOut, size_t bytesToRead, size_t* pBytesRead)
typedef ma_result (* ma_decoder_seek_proc)(ma_decoder* pDecoder, ma_int64 byteOffset, ma_seek_origin origin)
typedef ma_result (* ma_decoder_tell_proc)(ma_decoder* pDecoder, ma_int64* pCursor)

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_channel* pChannelMap
    ma_channel_mix_mode channelMixMode
    ma_dither_mode ditherMode
    ma_resampler_config resampling
    ma_allocation_callbacks allocationCallbacks
    ma_encoding_format encodingFormat
    ma_uint32 seekPointCount
    ma_decoding_backend_vtable** ppCustomBackendVTables
    ma_uint32 customBackendCount
    void* pCustomBackendUserData
} ma_decoder_config

struct ma_decoder
{
    ma_data_source_base ds
    ma_data_source* pBackend
    const ma_decoding_backend_vtable* pBackendVTable
    void* pBackendUserData
    ma_decoder_read_proc onRead
    ma_decoder_seek_proc onSeek
    ma_decoder_tell_proc onTell
    void* pUserData
    ma_uint64 readPointerInPCMFrames
    ma_format outputFormat
    ma_uint32 outputChannels
    ma_uint32 outputSampleRate
    ma_data_converter converter
    void* pInputCache
    ma_uint64 inputCacheCap
    ma_uint64 inputCacheConsumed
    ma_uint64 inputCacheRemaining
    ma_allocation_callbacks allocationCallbacks
    union
    {
        struct
        {
            ma_vfs* pVFS
            ma_vfs_file file
        } vfs
        struct
        {
            const ma_uint8* pData
            size_t dataSize
            size_t currentReadPos
        } memory
    } data
}

ma_decoder_config ma_decoder_config_init(ma_format outputFormat, ma_uint32 outputChannels, ma_uint32 outputSampleRate)
ma_decoder_config ma_decoder_config_init_default(void)

ma_result ma_decoder_init(ma_decoder_read_proc onRead, ma_decoder_seek_proc onSeek, void* pUserData, const ma_decoder_config* pConfig, ma_decoder* pDecoder)
ma_result ma_decoder_init_memory(const void* pData, size_t dataSize, const ma_decoder_config* pConfig, ma_decoder* pDecoder)
ma_result ma_decoder_init_vfs(ma_vfs* pVFS, const char* pFilePath, const ma_decoder_config* pConfig, ma_decoder* pDecoder)
ma_result ma_decoder_init_vfs_w(ma_vfs* pVFS, const wchar_t* pFilePath, const ma_decoder_config* pConfig, ma_decoder* pDecoder)
ma_result ma_decoder_init_file(const char* pFilePath, const ma_decoder_config* pConfig, ma_decoder* pDecoder)
ma_result ma_decoder_init_file_w(const wchar_t* pFilePath, const ma_decoder_config* pConfig, ma_decoder* pDecoder)




ma_result ma_decoder_uninit(ma_decoder* pDecoder)






ma_result ma_decoder_read_pcm_frames(ma_decoder* pDecoder, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)






ma_result ma_decoder_seek_to_pcm_frame(ma_decoder* pDecoder, ma_uint64 frameIndex)




ma_result ma_decoder_get_data_format(ma_decoder* pDecoder, ma_format* pFormat, ma_uint32* pChannels, ma_uint32* pSampleRate, ma_channel* pChannelMap, size_t channelMapCap)




ma_result ma_decoder_get_cursor_in_pcm_frames(ma_decoder* pDecoder, ma_uint64* pCursor)

ma_result ma_decoder_get_length_in_pcm_frames(ma_decoder* pDecoder, ma_uint64* pLength)

ma_result ma_decoder_get_available_frames(ma_decoder* pDecoder, ma_uint64* pAvailableFrames)





ma_result ma_decode_from_vfs(ma_vfs* pVFS, const char* pFilePath, ma_decoder_config* pConfig, ma_uint64* pFrameCountOut, void** ppPCMFramesOut)
ma_result ma_decode_file(const char* pFilePath, ma_decoder_config* pConfig, ma_uint64* pFrameCountOut, void** ppPCMFramesOut)
ma_result ma_decode_memory(const void* pData, size_t dataSize, ma_decoder_config* pConfig, ma_uint64* pFrameCountOut, void** ppPCMFramesOut)

typedef struct ma_encoder ma_encoder

typedef ma_result (* ma_encoder_write_proc) (ma_encoder* pEncoder, const void* pBufferIn, size_t bytesToWrite, size_t* pBytesWritten)
typedef ma_result (* ma_encoder_seek_proc) (ma_encoder* pEncoder, ma_int64 offset, ma_seek_origin origin)
typedef ma_result (* ma_encoder_init_proc) (ma_encoder* pEncoder)
typedef void (* ma_encoder_uninit_proc) (ma_encoder* pEncoder)
typedef ma_result (* ma_encoder_write_pcm_frames_proc)(ma_encoder* pEncoder, const void* pFramesIn, ma_uint64 frameCount, ma_uint64* pFramesWritten)

typedef struct
{
    ma_encoding_format encodingFormat
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_allocation_callbacks allocationCallbacks
} ma_encoder_config

ma_encoder_config ma_encoder_config_init(ma_encoding_format encodingFormat, ma_format format, ma_uint32 channels, ma_uint32 sampleRate)

struct ma_encoder
{
    ma_encoder_config config
    ma_encoder_write_proc onWrite
    ma_encoder_seek_proc onSeek
    ma_encoder_init_proc onInit
    ma_encoder_uninit_proc onUninit
    ma_encoder_write_pcm_frames_proc onWritePCMFrames
    void* pUserData
    void* pInternalEncoder
    union
    {
        struct
        {
            ma_vfs* pVFS
            ma_vfs_file file
        } vfs
    } data
}

ma_result ma_encoder_init(ma_encoder_write_proc onWrite, ma_encoder_seek_proc onSeek, void* pUserData, const ma_encoder_config* pConfig, ma_encoder* pEncoder)
ma_result ma_encoder_init_vfs(ma_vfs* pVFS, const char* pFilePath, const ma_encoder_config* pConfig, ma_encoder* pEncoder)
ma_result ma_encoder_init_vfs_w(ma_vfs* pVFS, const wchar_t* pFilePath, const ma_encoder_config* pConfig, ma_encoder* pEncoder)
ma_result ma_encoder_init_file(const char* pFilePath, const ma_encoder_config* pConfig, ma_encoder* pEncoder)
ma_result ma_encoder_init_file_w(const wchar_t* pFilePath, const ma_encoder_config* pConfig, ma_encoder* pEncoder)
void ma_encoder_uninit(ma_encoder* pEncoder)
ma_result ma_encoder_write_pcm_frames(ma_encoder* pEncoder, const void* pFramesIn, ma_uint64 frameCount, ma_uint64* pFramesWritten)

typedef enum
{
    ma_waveform_type_sine,
    ma_waveform_type_square,
    ma_waveform_type_triangle,
    ma_waveform_type_sawtooth
} ma_waveform_type

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_waveform_type type
    double amplitude
    double frequency
} ma_waveform_config

ma_waveform_config ma_waveform_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, ma_waveform_type type, double amplitude, double frequency)

typedef struct
{
    ma_data_source_base ds
    ma_waveform_config config
    double advance
    double time
} ma_waveform

ma_result ma_waveform_init(const ma_waveform_config* pConfig, ma_waveform* pWaveform)
void ma_waveform_uninit(ma_waveform* pWaveform)
ma_result ma_waveform_read_pcm_frames(ma_waveform* pWaveform, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_result ma_waveform_seek_to_pcm_frame(ma_waveform* pWaveform, ma_uint64 frameIndex)
ma_result ma_waveform_set_amplitude(ma_waveform* pWaveform, double amplitude)
ma_result ma_waveform_set_frequency(ma_waveform* pWaveform, double frequency)
ma_result ma_waveform_set_type(ma_waveform* pWaveform, ma_waveform_type type)
ma_result ma_waveform_set_sample_rate(ma_waveform* pWaveform, ma_uint32 sampleRate)

typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    double dutyCycle
    double amplitude
    double frequency
} ma_pulsewave_config

ma_pulsewave_config ma_pulsewave_config_init(ma_format format, ma_uint32 channels, ma_uint32 sampleRate, double dutyCycle, double amplitude, double frequency)

typedef struct
{
    ma_waveform waveform
    ma_pulsewave_config config
} ma_pulsewave

ma_result ma_pulsewave_init(const ma_pulsewave_config* pConfig, ma_pulsewave* pWaveform)
void ma_pulsewave_uninit(ma_pulsewave* pWaveform)
ma_result ma_pulsewave_read_pcm_frames(ma_pulsewave* pWaveform, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_result ma_pulsewave_seek_to_pcm_frame(ma_pulsewave* pWaveform, ma_uint64 frameIndex)
ma_result ma_pulsewave_set_amplitude(ma_pulsewave* pWaveform, double amplitude)
ma_result ma_pulsewave_set_frequency(ma_pulsewave* pWaveform, double frequency)
ma_result ma_pulsewave_set_sample_rate(ma_pulsewave* pWaveform, ma_uint32 sampleRate)
ma_result ma_pulsewave_set_duty_cycle(ma_pulsewave* pWaveform, double dutyCycle)

typedef enum
{
    ma_noise_type_white,
    ma_noise_type_pink,
    ma_noise_type_brownian
} ma_noise_type


typedef struct
{
    ma_format format
    ma_uint32 channels
    ma_noise_type type
    ma_int32 seed
    double amplitude
    ma_bool32 duplicateChannels
} ma_noise_config

ma_noise_config ma_noise_config_init(ma_format format, ma_uint32 channels, ma_noise_type type, ma_int32 seed, double amplitude)

typedef struct
{
    ma_data_source_base ds
    ma_noise_config config
    ma_lcg lcg
    union
    {
        struct
        {
            double** bin
            double* accumulation
            ma_uint32* counter
        } pink
        struct
        {
            double* accumulation
        } brownian
    } state


    void* _pHeap
    ma_bool32 _ownsHeap
} ma_noise

ma_result ma_noise_get_heap_size(const ma_noise_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_noise_init_preallocated(const ma_noise_config* pConfig, void* pHeap, ma_noise* pNoise)
ma_result ma_noise_init(const ma_noise_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_noise* pNoise)
void ma_noise_uninit(ma_noise* pNoise, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_noise_read_pcm_frames(ma_noise* pNoise, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_result ma_noise_set_amplitude(ma_noise* pNoise, double amplitude)
ma_result ma_noise_set_seed(ma_noise* pNoise, ma_int32 seed)
ma_result ma_noise_set_type(ma_noise* pNoise, ma_noise_type type)

typedef struct ma_resource_manager ma_resource_manager
typedef struct ma_resource_manager_data_buffer_node ma_resource_manager_data_buffer_node
typedef struct ma_resource_manager_data_buffer ma_resource_manager_data_buffer
typedef struct ma_resource_manager_data_stream ma_resource_manager_data_stream
typedef struct ma_resource_manager_data_source ma_resource_manager_data_source

typedef enum
{
    MA_RESOURCE_MANAGER_DATA_SOURCE_FLAG_STREAM = 0x00000001,
    MA_RESOURCE_MANAGER_DATA_SOURCE_FLAG_DECODE = 0x00000002,
    MA_RESOURCE_MANAGER_DATA_SOURCE_FLAG_ASYNC = 0x00000004,
    MA_RESOURCE_MANAGER_DATA_SOURCE_FLAG_WAIT_INIT = 0x00000008,
    MA_RESOURCE_MANAGER_DATA_SOURCE_FLAG_UNKNOWN_LENGTH = 0x00000010
} ma_resource_manager_data_source_flags





typedef struct
{
    ma_async_notification* pNotification
    ma_fence* pFence
} ma_resource_manager_pipeline_stage_notification

typedef struct
{
    ma_resource_manager_pipeline_stage_notification init
    ma_resource_manager_pipeline_stage_notification done
} ma_resource_manager_pipeline_notifications

ma_resource_manager_pipeline_notifications ma_resource_manager_pipeline_notifications_init(void)

typedef enum
{

    MA_RESOURCE_MANAGER_FLAG_NON_BLOCKING = 0x00000001,


    MA_RESOURCE_MANAGER_FLAG_NO_THREADING = 0x00000002
} ma_resource_manager_flags

typedef struct
{
    const char* pFilePath
    const wchar_t* pFilePathW
    const ma_resource_manager_pipeline_notifications* pNotifications
    ma_uint64 initialSeekPointInPCMFrames
    ma_uint64 rangeBegInPCMFrames
    ma_uint64 rangeEndInPCMFrames
    ma_uint64 loopPointBegInPCMFrames
    ma_uint64 loopPointEndInPCMFrames
    ma_bool32 isLooping
    ma_uint32 flags
} ma_resource_manager_data_source_config

ma_resource_manager_data_source_config ma_resource_manager_data_source_config_init(void)


typedef enum
{
    ma_resource_manager_data_supply_type_unknown = 0,
    ma_resource_manager_data_supply_type_encoded,
    ma_resource_manager_data_supply_type_decoded,
    ma_resource_manager_data_supply_type_decoded_paged
} ma_resource_manager_data_supply_type

typedef struct
{
    _Alignas(4) ma_resource_manager_data_supply_type type
    union
    {
        struct
        {
            const void* pData
            size_t sizeInBytes
        } encoded
        struct
        {
            const void* pData
            ma_uint64 totalFrameCount
            ma_uint64 decodedFrameCount
            ma_format format
            ma_uint32 channels
            ma_uint32 sampleRate
        } decoded
        struct
        {
            ma_paged_audio_buffer_data data
            ma_uint64 decodedFrameCount
            ma_uint32 sampleRate
        } decodedPaged
    } backend
} ma_resource_manager_data_supply

struct ma_resource_manager_data_buffer_node
{
    ma_uint32 hashedName32
    ma_uint32 refCount
    _Alignas(4) ma_result result
    _Alignas(4) ma_uint32 executionCounter
    _Alignas(4) ma_uint32 executionPointer
    ma_bool32 isDataOwnedByResourceManager
    ma_resource_manager_data_supply data
    ma_resource_manager_data_buffer_node* pParent
    ma_resource_manager_data_buffer_node* pChildLo
    ma_resource_manager_data_buffer_node* pChildHi
}

struct ma_resource_manager_data_buffer
{
    ma_data_source_base ds
    ma_resource_manager* pResourceManager
    ma_resource_manager_data_buffer_node* pNode
    ma_uint32 flags
    _Alignas(4) ma_uint32 executionCounter
    _Alignas(4) ma_uint32 executionPointer
    ma_uint64 seekTargetInPCMFrames
    ma_bool32 seekToCursorOnNextRead
    _Alignas(4) ma_result result
    _Alignas(4) ma_bool32 isLooping
    ma_atomic_bool32 isConnectorInitialized
    union
    {
        ma_decoder decoder
        ma_audio_buffer buffer
        ma_paged_audio_buffer pagedBuffer
    } connector
}

struct ma_resource_manager_data_stream
{
    ma_data_source_base ds
    ma_resource_manager* pResourceManager
    ma_uint32 flags
    ma_decoder decoder
    ma_bool32 isDecoderInitialized
    ma_uint64 totalLengthInPCMFrames
    ma_uint32 relativeCursor
    _Alignas(8) ma_uint64 absoluteCursor
    ma_uint32 currentPageIndex
    _Alignas(4) ma_uint32 executionCounter
    _Alignas(4) ma_uint32 executionPointer


    _Alignas(4) ma_bool32 isLooping


    void* pPageData
    _Alignas(4) ma_uint32 pageFrameCount[2]


    _Alignas(4) ma_result result
    _Alignas(4) ma_bool32 isDecoderAtEnd
    _Alignas(4) ma_bool32 isPageValid[2]
    _Alignas(4) ma_bool32 seekCounter
}

struct ma_resource_manager_data_source
{
    union
    {
        ma_resource_manager_data_buffer buffer
        ma_resource_manager_data_stream stream
    } backend

    ma_uint32 flags
    _Alignas(4) ma_uint32 executionCounter
    _Alignas(4) ma_uint32 executionPointer
}

typedef struct
{
    ma_allocation_callbacks allocationCallbacks
    ma_log* pLog
    ma_format decodedFormat
    ma_uint32 decodedChannels
    ma_uint32 decodedSampleRate
    ma_uint32 jobThreadCount
    size_t jobThreadStackSize
    ma_uint32 jobQueueCapacity
    ma_uint32 flags
    ma_vfs* pVFS
    ma_decoding_backend_vtable** ppCustomDecodingBackendVTables
    ma_uint32 customDecodingBackendCount
    void* pCustomDecodingBackendUserData
} ma_resource_manager_config

ma_resource_manager_config ma_resource_manager_config_init(void)

struct ma_resource_manager
{
    ma_resource_manager_config config
    ma_resource_manager_data_buffer_node* pRootDataBufferNode

    ma_mutex dataBufferBSTLock
    ma_thread jobThreads[64]

    ma_job_queue jobQueue
    ma_default_vfs defaultVFS
    ma_log log
}


ma_result ma_resource_manager_init(const ma_resource_manager_config* pConfig, ma_resource_manager* pResourceManager)
void ma_resource_manager_uninit(ma_resource_manager* pResourceManager)
ma_log* ma_resource_manager_get_log(ma_resource_manager* pResourceManager)


ma_result ma_resource_manager_register_file(ma_resource_manager* pResourceManager, const char* pFilePath, ma_uint32 flags)
ma_result ma_resource_manager_register_file_w(ma_resource_manager* pResourceManager, const wchar_t* pFilePath, ma_uint32 flags)
ma_result ma_resource_manager_register_decoded_data(ma_resource_manager* pResourceManager, const char* pName, const void* pData, ma_uint64 frameCount, ma_format format, ma_uint32 channels, ma_uint32 sampleRate)
ma_result ma_resource_manager_register_decoded_data_w(ma_resource_manager* pResourceManager, const wchar_t* pName, const void* pData, ma_uint64 frameCount, ma_format format, ma_uint32 channels, ma_uint32 sampleRate)
ma_result ma_resource_manager_register_encoded_data(ma_resource_manager* pResourceManager, const char* pName, const void* pData, size_t sizeInBytes)
ma_result ma_resource_manager_register_encoded_data_w(ma_resource_manager* pResourceManager, const wchar_t* pName, const void* pData, size_t sizeInBytes)
ma_result ma_resource_manager_unregister_file(ma_resource_manager* pResourceManager, const char* pFilePath)
ma_result ma_resource_manager_unregister_file_w(ma_resource_manager* pResourceManager, const wchar_t* pFilePath)
ma_result ma_resource_manager_unregister_data(ma_resource_manager* pResourceManager, const char* pName)
ma_result ma_resource_manager_unregister_data_w(ma_resource_manager* pResourceManager, const wchar_t* pName)


ma_result ma_resource_manager_data_buffer_init_ex(ma_resource_manager* pResourceManager, const ma_resource_manager_data_source_config* pConfig, ma_resource_manager_data_buffer* pDataBuffer)
ma_result ma_resource_manager_data_buffer_init(ma_resource_manager* pResourceManager, const char* pFilePath, ma_uint32 flags, const ma_resource_manager_pipeline_notifications* pNotifications, ma_resource_manager_data_buffer* pDataBuffer)
ma_result ma_resource_manager_data_buffer_init_w(ma_resource_manager* pResourceManager, const wchar_t* pFilePath, ma_uint32 flags, const ma_resource_manager_pipeline_notifications* pNotifications, ma_resource_manager_data_buffer* pDataBuffer)
ma_result ma_resource_manager_data_buffer_init_copy(ma_resource_manager* pResourceManager, const ma_resource_manager_data_buffer* pExistingDataBuffer, ma_resource_manager_data_buffer* pDataBuffer)
ma_result ma_resource_manager_data_buffer_uninit(ma_resource_manager_data_buffer* pDataBuffer)
ma_result ma_resource_manager_data_buffer_read_pcm_frames(ma_resource_manager_data_buffer* pDataBuffer, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_result ma_resource_manager_data_buffer_seek_to_pcm_frame(ma_resource_manager_data_buffer* pDataBuffer, ma_uint64 frameIndex)
ma_result ma_resource_manager_data_buffer_get_data_format(ma_resource_manager_data_buffer* pDataBuffer, ma_format* pFormat, ma_uint32* pChannels, ma_uint32* pSampleRate, ma_channel* pChannelMap, size_t channelMapCap)
ma_result ma_resource_manager_data_buffer_get_cursor_in_pcm_frames(ma_resource_manager_data_buffer* pDataBuffer, ma_uint64* pCursor)
ma_result ma_resource_manager_data_buffer_get_length_in_pcm_frames(ma_resource_manager_data_buffer* pDataBuffer, ma_uint64* pLength)
ma_result ma_resource_manager_data_buffer_result(const ma_resource_manager_data_buffer* pDataBuffer)
ma_result ma_resource_manager_data_buffer_set_looping(ma_resource_manager_data_buffer* pDataBuffer, ma_bool32 isLooping)
ma_bool32 ma_resource_manager_data_buffer_is_looping(const ma_resource_manager_data_buffer* pDataBuffer)
ma_result ma_resource_manager_data_buffer_get_available_frames(ma_resource_manager_data_buffer* pDataBuffer, ma_uint64* pAvailableFrames)


ma_result ma_resource_manager_data_stream_init_ex(ma_resource_manager* pResourceManager, const ma_resource_manager_data_source_config* pConfig, ma_resource_manager_data_stream* pDataStream)
ma_result ma_resource_manager_data_stream_init(ma_resource_manager* pResourceManager, const char* pFilePath, ma_uint32 flags, const ma_resource_manager_pipeline_notifications* pNotifications, ma_resource_manager_data_stream* pDataStream)
ma_result ma_resource_manager_data_stream_init_w(ma_resource_manager* pResourceManager, const wchar_t* pFilePath, ma_uint32 flags, const ma_resource_manager_pipeline_notifications* pNotifications, ma_resource_manager_data_stream* pDataStream)
ma_result ma_resource_manager_data_stream_uninit(ma_resource_manager_data_stream* pDataStream)
ma_result ma_resource_manager_data_stream_read_pcm_frames(ma_resource_manager_data_stream* pDataStream, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_result ma_resource_manager_data_stream_seek_to_pcm_frame(ma_resource_manager_data_stream* pDataStream, ma_uint64 frameIndex)
ma_result ma_resource_manager_data_stream_get_data_format(ma_resource_manager_data_stream* pDataStream, ma_format* pFormat, ma_uint32* pChannels, ma_uint32* pSampleRate, ma_channel* pChannelMap, size_t channelMapCap)
ma_result ma_resource_manager_data_stream_get_cursor_in_pcm_frames(ma_resource_manager_data_stream* pDataStream, ma_uint64* pCursor)
ma_result ma_resource_manager_data_stream_get_length_in_pcm_frames(ma_resource_manager_data_stream* pDataStream, ma_uint64* pLength)
ma_result ma_resource_manager_data_stream_result(const ma_resource_manager_data_stream* pDataStream)
ma_result ma_resource_manager_data_stream_set_looping(ma_resource_manager_data_stream* pDataStream, ma_bool32 isLooping)
ma_bool32 ma_resource_manager_data_stream_is_looping(const ma_resource_manager_data_stream* pDataStream)
ma_result ma_resource_manager_data_stream_get_available_frames(ma_resource_manager_data_stream* pDataStream, ma_uint64* pAvailableFrames)


ma_result ma_resource_manager_data_source_init_ex(ma_resource_manager* pResourceManager, const ma_resource_manager_data_source_config* pConfig, ma_resource_manager_data_source* pDataSource)
ma_result ma_resource_manager_data_source_init(ma_resource_manager* pResourceManager, const char* pName, ma_uint32 flags, const ma_resource_manager_pipeline_notifications* pNotifications, ma_resource_manager_data_source* pDataSource)
ma_result ma_resource_manager_data_source_init_w(ma_resource_manager* pResourceManager, const wchar_t* pName, ma_uint32 flags, const ma_resource_manager_pipeline_notifications* pNotifications, ma_resource_manager_data_source* pDataSource)
ma_result ma_resource_manager_data_source_init_copy(ma_resource_manager* pResourceManager, const ma_resource_manager_data_source* pExistingDataSource, ma_resource_manager_data_source* pDataSource)
ma_result ma_resource_manager_data_source_uninit(ma_resource_manager_data_source* pDataSource)
ma_result ma_resource_manager_data_source_read_pcm_frames(ma_resource_manager_data_source* pDataSource, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_result ma_resource_manager_data_source_seek_to_pcm_frame(ma_resource_manager_data_source* pDataSource, ma_uint64 frameIndex)
ma_result ma_resource_manager_data_source_get_data_format(ma_resource_manager_data_source* pDataSource, ma_format* pFormat, ma_uint32* pChannels, ma_uint32* pSampleRate, ma_channel* pChannelMap, size_t channelMapCap)
ma_result ma_resource_manager_data_source_get_cursor_in_pcm_frames(ma_resource_manager_data_source* pDataSource, ma_uint64* pCursor)
ma_result ma_resource_manager_data_source_get_length_in_pcm_frames(ma_resource_manager_data_source* pDataSource, ma_uint64* pLength)
ma_result ma_resource_manager_data_source_result(const ma_resource_manager_data_source* pDataSource)
ma_result ma_resource_manager_data_source_set_looping(ma_resource_manager_data_source* pDataSource, ma_bool32 isLooping)
ma_bool32 ma_resource_manager_data_source_is_looping(const ma_resource_manager_data_source* pDataSource)
ma_result ma_resource_manager_data_source_get_available_frames(ma_resource_manager_data_source* pDataSource, ma_uint64* pAvailableFrames)


ma_result ma_resource_manager_post_job(ma_resource_manager* pResourceManager, const ma_job* pJob)
ma_result ma_resource_manager_post_job_quit(ma_resource_manager* pResourceManager)
ma_result ma_resource_manager_next_job(ma_resource_manager* pResourceManager, ma_job* pJob)
ma_result ma_resource_manager_process_job(ma_resource_manager* pResourceManager, ma_job* pJob)
ma_result ma_resource_manager_process_next_job(ma_resource_manager* pResourceManager)

typedef struct ma_node_graph ma_node_graph
typedef void ma_node



typedef enum
{
    MA_NODE_FLAG_PASSTHROUGH = 0x00000001,
    MA_NODE_FLAG_CONTINUOUS_PROCESSING = 0x00000002,
    MA_NODE_FLAG_ALLOW_NULL_INPUT = 0x00000004,
    MA_NODE_FLAG_DIFFERENT_PROCESSING_RATES = 0x00000008,
    MA_NODE_FLAG_SILENT_OUTPUT = 0x00000010
} ma_node_flags



typedef enum
{
    ma_node_state_started = 0,
    ma_node_state_stopped = 1
} ma_node_state


typedef struct
{
    void (* onProcess)(ma_node* pNode, const float** ppFramesIn, ma_uint32* pFrameCountIn, float** ppFramesOut, ma_uint32* pFrameCountOut)
    ma_result (* onGetRequiredInputFrameCount)(ma_node* pNode, ma_uint32 outputFrameCount, ma_uint32* pInputFrameCount)





    ma_uint8 inputBusCount





    ma_uint8 outputBusCount





    ma_uint32 flags
} ma_node_vtable

typedef struct
{
    const ma_node_vtable* vtable
    ma_node_state initialState
    ma_uint32 inputBusCount
    ma_uint32 outputBusCount
    const ma_uint32* pInputChannels
    const ma_uint32* pOutputChannels
} ma_node_config

ma_node_config ma_node_config_init(void)






typedef struct ma_node_output_bus ma_node_output_bus
struct ma_node_output_bus
{

    ma_node* pNode
    ma_uint8 outputBusIndex
    ma_uint8 channels


    ma_uint8 inputNodeInputBusIndex
    _Alignas(4) ma_uint32 flags
    _Alignas(4) ma_uint32 refCount
    _Alignas(4) ma_bool32 isAttached
    _Alignas(4) ma_spinlock lock
    _Alignas(4) float volume
    _Alignas(8) ma_node_output_bus* pNext
    _Alignas(8) ma_node_output_bus* pPrev
    _Alignas(8) ma_node* pInputNode
}





typedef struct ma_node_input_bus ma_node_input_bus
struct ma_node_input_bus
{

    ma_node_output_bus head
    _Alignas(4) ma_uint32 nextCounter
    _Alignas(4) ma_spinlock lock


    ma_uint8 channels
}


typedef struct ma_node_base ma_node_base
struct ma_node_base
{

    ma_node_graph* pNodeGraph
    const ma_node_vtable* vtable
    float* pCachedData
    ma_uint16 cachedDataCapInFramesPerBus


    ma_uint16 cachedFrameCountOut
    ma_uint16 cachedFrameCountIn
    ma_uint16 consumedFrameCountIn


    _Alignas(4) ma_node_state state
    _Alignas(8) ma_uint64 stateTimes[2]
    _Alignas(8) ma_uint64 localTime
    ma_uint32 inputBusCount
    ma_uint32 outputBusCount
    ma_node_input_bus* pInputBuses
    ma_node_output_bus* pOutputBuses


    ma_node_input_bus _inputBuses[2]
    ma_node_output_bus _outputBuses[2]
    void* _pHeap
    ma_bool32 _ownsHeap
}

ma_result ma_node_get_heap_size(ma_node_graph* pNodeGraph, const ma_node_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_node_init_preallocated(ma_node_graph* pNodeGraph, const ma_node_config* pConfig, void* pHeap, ma_node* pNode)
ma_result ma_node_init(ma_node_graph* pNodeGraph, const ma_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_node* pNode)
void ma_node_uninit(ma_node* pNode, const ma_allocation_callbacks* pAllocationCallbacks)
ma_node_graph* ma_node_get_node_graph(const ma_node* pNode)
ma_uint32 ma_node_get_input_bus_count(const ma_node* pNode)
ma_uint32 ma_node_get_output_bus_count(const ma_node* pNode)
ma_uint32 ma_node_get_input_channels(const ma_node* pNode, ma_uint32 inputBusIndex)
ma_uint32 ma_node_get_output_channels(const ma_node* pNode, ma_uint32 outputBusIndex)
ma_result ma_node_attach_output_bus(ma_node* pNode, ma_uint32 outputBusIndex, ma_node* pOtherNode, ma_uint32 otherNodeInputBusIndex)
ma_result ma_node_detach_output_bus(ma_node* pNode, ma_uint32 outputBusIndex)
ma_result ma_node_detach_all_output_buses(ma_node* pNode)
ma_result ma_node_set_output_bus_volume(ma_node* pNode, ma_uint32 outputBusIndex, float volume)
float ma_node_get_output_bus_volume(const ma_node* pNode, ma_uint32 outputBusIndex)
ma_result ma_node_set_state(ma_node* pNode, ma_node_state state)
ma_node_state ma_node_get_state(const ma_node* pNode)
ma_result ma_node_set_state_time(ma_node* pNode, ma_node_state state, ma_uint64 globalTime)
ma_uint64 ma_node_get_state_time(const ma_node* pNode, ma_node_state state)
ma_node_state ma_node_get_state_by_time(const ma_node* pNode, ma_uint64 globalTime)
ma_node_state ma_node_get_state_by_time_range(const ma_node* pNode, ma_uint64 globalTimeBeg, ma_uint64 globalTimeEnd)
ma_uint64 ma_node_get_time(const ma_node* pNode)
ma_result ma_node_set_time(ma_node* pNode, ma_uint64 localTime)


typedef struct
{
    ma_uint32 channels
    ma_uint16 nodeCacheCapInFrames
} ma_node_graph_config

ma_node_graph_config ma_node_graph_config_init(ma_uint32 channels)


struct ma_node_graph
{

    ma_node_base base
    ma_node_base endpoint
    ma_uint16 nodeCacheCapInFrames


    _Alignas(4) ma_bool32 isReading
}

ma_result ma_node_graph_init(const ma_node_graph_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_node_graph* pNodeGraph)
void ma_node_graph_uninit(ma_node_graph* pNodeGraph, const ma_allocation_callbacks* pAllocationCallbacks)
ma_node* ma_node_graph_get_endpoint(ma_node_graph* pNodeGraph)
ma_result ma_node_graph_read_pcm_frames(ma_node_graph* pNodeGraph, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_uint32 ma_node_graph_get_channels(const ma_node_graph* pNodeGraph)
ma_uint64 ma_node_graph_get_time(const ma_node_graph* pNodeGraph)
ma_result ma_node_graph_set_time(ma_node_graph* pNodeGraph, ma_uint64 globalTime)




typedef struct
{
    ma_node_config nodeConfig
    ma_data_source* pDataSource
} ma_data_source_node_config

ma_data_source_node_config ma_data_source_node_config_init(ma_data_source* pDataSource)


typedef struct
{
    ma_node_base base
    ma_data_source* pDataSource
} ma_data_source_node

ma_result ma_data_source_node_init(ma_node_graph* pNodeGraph, const ma_data_source_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_data_source_node* pDataSourceNode)
void ma_data_source_node_uninit(ma_data_source_node* pDataSourceNode, const ma_allocation_callbacks* pAllocationCallbacks)
ma_result ma_data_source_node_set_looping(ma_data_source_node* pDataSourceNode, ma_bool32 isLooping)
ma_bool32 ma_data_source_node_is_looping(ma_data_source_node* pDataSourceNode)



typedef struct
{
    ma_node_config nodeConfig
    ma_uint32 channels
    ma_uint32 outputBusCount
} ma_splitter_node_config

ma_splitter_node_config ma_splitter_node_config_init(ma_uint32 channels)


typedef struct
{
    ma_node_base base
} ma_splitter_node

ma_result ma_splitter_node_init(ma_node_graph* pNodeGraph, const ma_splitter_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_splitter_node* pSplitterNode)
void ma_splitter_node_uninit(ma_splitter_node* pSplitterNode, const ma_allocation_callbacks* pAllocationCallbacks)





typedef struct
{
    ma_node_config nodeConfig
    ma_biquad_config biquad
} ma_biquad_node_config

ma_biquad_node_config ma_biquad_node_config_init(ma_uint32 channels, float b0, float b1, float b2, float a0, float a1, float a2)


typedef struct
{
    ma_node_base baseNode
    ma_biquad biquad
} ma_biquad_node

ma_result ma_biquad_node_init(ma_node_graph* pNodeGraph, const ma_biquad_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_biquad_node* pNode)
ma_result ma_biquad_node_reinit(const ma_biquad_config* pConfig, ma_biquad_node* pNode)
void ma_biquad_node_uninit(ma_biquad_node* pNode, const ma_allocation_callbacks* pAllocationCallbacks)





typedef struct
{
    ma_node_config nodeConfig
    ma_lpf_config lpf
} ma_lpf_node_config

ma_lpf_node_config ma_lpf_node_config_init(ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency, ma_uint32 order)


typedef struct
{
    ma_node_base baseNode
    ma_lpf lpf
} ma_lpf_node

ma_result ma_lpf_node_init(ma_node_graph* pNodeGraph, const ma_lpf_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_lpf_node* pNode)
ma_result ma_lpf_node_reinit(const ma_lpf_config* pConfig, ma_lpf_node* pNode)
void ma_lpf_node_uninit(ma_lpf_node* pNode, const ma_allocation_callbacks* pAllocationCallbacks)





typedef struct
{
    ma_node_config nodeConfig
    ma_hpf_config hpf
} ma_hpf_node_config

ma_hpf_node_config ma_hpf_node_config_init(ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency, ma_uint32 order)


typedef struct
{
    ma_node_base baseNode
    ma_hpf hpf
} ma_hpf_node

ma_result ma_hpf_node_init(ma_node_graph* pNodeGraph, const ma_hpf_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_hpf_node* pNode)
ma_result ma_hpf_node_reinit(const ma_hpf_config* pConfig, ma_hpf_node* pNode)
void ma_hpf_node_uninit(ma_hpf_node* pNode, const ma_allocation_callbacks* pAllocationCallbacks)





typedef struct
{
    ma_node_config nodeConfig
    ma_bpf_config bpf
} ma_bpf_node_config

ma_bpf_node_config ma_bpf_node_config_init(ma_uint32 channels, ma_uint32 sampleRate, double cutoffFrequency, ma_uint32 order)


typedef struct
{
    ma_node_base baseNode
    ma_bpf bpf
} ma_bpf_node

ma_result ma_bpf_node_init(ma_node_graph* pNodeGraph, const ma_bpf_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_bpf_node* pNode)
ma_result ma_bpf_node_reinit(const ma_bpf_config* pConfig, ma_bpf_node* pNode)
void ma_bpf_node_uninit(ma_bpf_node* pNode, const ma_allocation_callbacks* pAllocationCallbacks)





typedef struct
{
    ma_node_config nodeConfig
    ma_notch_config notch
} ma_notch_node_config

ma_notch_node_config ma_notch_node_config_init(ma_uint32 channels, ma_uint32 sampleRate, double q, double frequency)


typedef struct
{
    ma_node_base baseNode
    ma_notch2 notch
} ma_notch_node

ma_result ma_notch_node_init(ma_node_graph* pNodeGraph, const ma_notch_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_notch_node* pNode)
ma_result ma_notch_node_reinit(const ma_notch_config* pConfig, ma_notch_node* pNode)
void ma_notch_node_uninit(ma_notch_node* pNode, const ma_allocation_callbacks* pAllocationCallbacks)





typedef struct
{
    ma_node_config nodeConfig
    ma_peak_config peak
} ma_peak_node_config

ma_peak_node_config ma_peak_node_config_init(ma_uint32 channels, ma_uint32 sampleRate, double gainDB, double q, double frequency)


typedef struct
{
    ma_node_base baseNode
    ma_peak2 peak
} ma_peak_node

ma_result ma_peak_node_init(ma_node_graph* pNodeGraph, const ma_peak_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_peak_node* pNode)
ma_result ma_peak_node_reinit(const ma_peak_config* pConfig, ma_peak_node* pNode)
void ma_peak_node_uninit(ma_peak_node* pNode, const ma_allocation_callbacks* pAllocationCallbacks)





typedef struct
{
    ma_node_config nodeConfig
    ma_loshelf_config loshelf
} ma_loshelf_node_config

ma_loshelf_node_config ma_loshelf_node_config_init(ma_uint32 channels, ma_uint32 sampleRate, double gainDB, double q, double frequency)


typedef struct
{
    ma_node_base baseNode
    ma_loshelf2 loshelf
} ma_loshelf_node

ma_result ma_loshelf_node_init(ma_node_graph* pNodeGraph, const ma_loshelf_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_loshelf_node* pNode)
ma_result ma_loshelf_node_reinit(const ma_loshelf_config* pConfig, ma_loshelf_node* pNode)
void ma_loshelf_node_uninit(ma_loshelf_node* pNode, const ma_allocation_callbacks* pAllocationCallbacks)





typedef struct
{
    ma_node_config nodeConfig
    ma_hishelf_config hishelf
} ma_hishelf_node_config

ma_hishelf_node_config ma_hishelf_node_config_init(ma_uint32 channels, ma_uint32 sampleRate, double gainDB, double q, double frequency)


typedef struct
{
    ma_node_base baseNode
    ma_hishelf2 hishelf
} ma_hishelf_node

ma_result ma_hishelf_node_init(ma_node_graph* pNodeGraph, const ma_hishelf_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_hishelf_node* pNode)
ma_result ma_hishelf_node_reinit(const ma_hishelf_config* pConfig, ma_hishelf_node* pNode)
void ma_hishelf_node_uninit(ma_hishelf_node* pNode, const ma_allocation_callbacks* pAllocationCallbacks)


typedef struct
{
    ma_node_config nodeConfig
    ma_delay_config delay
} ma_delay_node_config

ma_delay_node_config ma_delay_node_config_init(ma_uint32 channels, ma_uint32 sampleRate, ma_uint32 delayInFrames, float decay)


typedef struct
{
    ma_node_base baseNode
    ma_delay delay
} ma_delay_node

ma_result ma_delay_node_init(ma_node_graph* pNodeGraph, const ma_delay_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_delay_node* pDelayNode)
void ma_delay_node_uninit(ma_delay_node* pDelayNode, const ma_allocation_callbacks* pAllocationCallbacks)
void ma_delay_node_set_wet(ma_delay_node* pDelayNode, float value)
float ma_delay_node_get_wet(const ma_delay_node* pDelayNode)
void ma_delay_node_set_dry(ma_delay_node* pDelayNode, float value)
float ma_delay_node_get_dry(const ma_delay_node* pDelayNode)
void ma_delay_node_set_decay(ma_delay_node* pDelayNode, float value)
float ma_delay_node_get_decay(const ma_delay_node* pDelayNode)

typedef struct ma_engine ma_engine
typedef struct ma_sound ma_sound



typedef enum
{

    MA_SOUND_FLAG_STREAM = 0x00000001,
    MA_SOUND_FLAG_DECODE = 0x00000002,
    MA_SOUND_FLAG_ASYNC = 0x00000004,
    MA_SOUND_FLAG_WAIT_INIT = 0x00000008,
    MA_SOUND_FLAG_UNKNOWN_LENGTH = 0x00000010,


    MA_SOUND_FLAG_NO_DEFAULT_ATTACHMENT = 0x00001000,
    MA_SOUND_FLAG_NO_PITCH = 0x00002000,
    MA_SOUND_FLAG_NO_SPATIALIZATION = 0x00004000
} ma_sound_flags







typedef enum
{
    ma_engine_node_type_sound,
    ma_engine_node_type_group
} ma_engine_node_type

typedef struct
{
    ma_engine* pEngine
    ma_engine_node_type type
    ma_uint32 channelsIn
    ma_uint32 channelsOut
    ma_uint32 sampleRate
    ma_uint32 volumeSmoothTimeInPCMFrames
    ma_mono_expansion_mode monoExpansionMode
    ma_bool8 isPitchDisabled
    ma_bool8 isSpatializationDisabled
    ma_uint8 pinnedListenerIndex
} ma_engine_node_config

ma_engine_node_config ma_engine_node_config_init(ma_engine* pEngine, ma_engine_node_type type, ma_uint32 flags)



typedef struct
{
    ma_node_base baseNode
    ma_engine* pEngine
    ma_uint32 sampleRate
    ma_uint32 volumeSmoothTimeInPCMFrames
    ma_mono_expansion_mode monoExpansionMode
    ma_fader fader
    ma_linear_resampler resampler
    ma_spatializer spatializer
    ma_panner panner
    ma_gainer volumeGainer
    ma_atomic_float volume
    _Alignas(4) float pitch
    float oldPitch
    float oldDopplerPitch
    _Alignas(4) ma_bool32 isPitchDisabled
    _Alignas(4) ma_bool32 isSpatializationDisabled
    _Alignas(4) ma_uint32 pinnedListenerIndex


    struct
    {
        ma_atomic_float volumeBeg
        ma_atomic_float volumeEnd
        ma_atomic_uint64 fadeLengthInFrames
        ma_atomic_uint64 absoluteGlobalTimeInFrames
    } fadeSettings


    ma_bool8 _ownsHeap
    void* _pHeap
} ma_engine_node

ma_result ma_engine_node_get_heap_size(const ma_engine_node_config* pConfig, size_t* pHeapSizeInBytes)
ma_result ma_engine_node_init_preallocated(const ma_engine_node_config* pConfig, void* pHeap, ma_engine_node* pEngineNode)
ma_result ma_engine_node_init(const ma_engine_node_config* pConfig, const ma_allocation_callbacks* pAllocationCallbacks, ma_engine_node* pEngineNode)
void ma_engine_node_uninit(ma_engine_node* pEngineNode, const ma_allocation_callbacks* pAllocationCallbacks)





typedef void (* ma_sound_end_proc)(void* pUserData, ma_sound* pSound)

typedef struct
{
    const char* pFilePath
    const wchar_t* pFilePathW
    ma_data_source* pDataSource
    ma_node* pInitialAttachment
    ma_uint32 initialAttachmentInputBusIndex
    ma_uint32 channelsIn
    ma_uint32 channelsOut
    ma_mono_expansion_mode monoExpansionMode
    ma_uint32 flags
    ma_uint32 volumeSmoothTimeInPCMFrames
    ma_uint64 initialSeekPointInPCMFrames
    ma_uint64 rangeBegInPCMFrames
    ma_uint64 rangeEndInPCMFrames
    ma_uint64 loopPointBegInPCMFrames
    ma_uint64 loopPointEndInPCMFrames
    ma_bool32 isLooping
    ma_sound_end_proc endCallback
    void* pEndCallbackUserData

    ma_resource_manager_pipeline_notifications initNotifications

    ma_fence* pDoneFence
} ma_sound_config

ma_sound_config ma_sound_config_init(void)
ma_sound_config ma_sound_config_init_2(ma_engine* pEngine)

struct ma_sound
{
    ma_engine_node engineNode
    ma_data_source* pDataSource
    _Alignas(8) ma_uint64 seekTarget
    _Alignas(4) ma_bool32 atEnd
    ma_sound_end_proc endCallback
    void* pEndCallbackUserData
    ma_bool8 ownsDataSource






    ma_resource_manager_data_source* pResourceManagerDataSource

}


typedef struct ma_sound_inlined ma_sound_inlined
struct ma_sound_inlined
{
    ma_sound sound
    ma_sound_inlined* pNext
    ma_sound_inlined* pPrev
}


typedef ma_sound_config ma_sound_group_config
typedef ma_sound ma_sound_group

ma_sound_group_config ma_sound_group_config_init(void)
ma_sound_group_config ma_sound_group_config_init_2(ma_engine* pEngine)

typedef void (* ma_engine_process_proc)(void* pUserData, float* pFramesOut, ma_uint64 frameCount)

typedef struct
{

    ma_resource_manager* pResourceManager


    ma_context* pContext
    ma_device* pDevice
    ma_device_id* pPlaybackDeviceID
    ma_device_data_proc dataCallback
    ma_device_notification_proc notificationCallback

    ma_log* pLog
    ma_uint32 listenerCount
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_uint32 periodSizeInFrames
    ma_uint32 periodSizeInMilliseconds
    ma_uint32 gainSmoothTimeInFrames
    ma_uint32 gainSmoothTimeInMilliseconds
    ma_uint32 defaultVolumeSmoothTimeInPCMFrames
    ma_allocation_callbacks allocationCallbacks
    ma_bool32 noAutoStart
    ma_bool32 noDevice
    ma_mono_expansion_mode monoExpansionMode
    ma_vfs* pResourceManagerVFS
    ma_engine_process_proc onProcess
    void* pProcessUserData
} ma_engine_config

ma_engine_config ma_engine_config_init(void)


struct ma_engine
{
    ma_node_graph nodeGraph

    ma_resource_manager* pResourceManager


    ma_device* pDevice

    ma_log* pLog
    ma_uint32 sampleRate
    ma_uint32 listenerCount
    ma_spatializer_listener listeners[4]
    ma_allocation_callbacks allocationCallbacks
    ma_bool8 ownsResourceManager
    ma_bool8 ownsDevice
    ma_spinlock inlinedSoundLock
    ma_sound_inlined* pInlinedSoundHead
    _Alignas(4) ma_uint32 inlinedSoundCount
    ma_uint32 gainSmoothTimeInFrames
    ma_uint32 defaultVolumeSmoothTimeInPCMFrames
    ma_mono_expansion_mode monoExpansionMode
    ma_engine_process_proc onProcess
    void* pProcessUserData
}

ma_result ma_engine_init(const ma_engine_config* pConfig, ma_engine* pEngine)
void ma_engine_uninit(ma_engine* pEngine)
ma_result ma_engine_read_pcm_frames(ma_engine* pEngine, void* pFramesOut, ma_uint64 frameCount, ma_uint64* pFramesRead)
ma_node_graph* ma_engine_get_node_graph(ma_engine* pEngine)

ma_resource_manager* ma_engine_get_resource_manager(ma_engine* pEngine)

ma_device* ma_engine_get_device(ma_engine* pEngine)
ma_log* ma_engine_get_log(ma_engine* pEngine)
ma_node* ma_engine_get_endpoint(ma_engine* pEngine)
ma_uint64 ma_engine_get_time_in_pcm_frames(const ma_engine* pEngine)
ma_uint64 ma_engine_get_time_in_milliseconds(const ma_engine* pEngine)
ma_result ma_engine_set_time_in_pcm_frames(ma_engine* pEngine, ma_uint64 globalTime)
ma_result ma_engine_set_time_in_milliseconds(ma_engine* pEngine, ma_uint64 globalTime)
ma_uint64 ma_engine_get_time(const ma_engine* pEngine)
ma_result ma_engine_set_time(ma_engine* pEngine, ma_uint64 globalTime)
ma_uint32 ma_engine_get_channels(const ma_engine* pEngine)
ma_uint32 ma_engine_get_sample_rate(const ma_engine* pEngine)

ma_result ma_engine_start(ma_engine* pEngine)
ma_result ma_engine_stop(ma_engine* pEngine)
ma_result ma_engine_set_volume(ma_engine* pEngine, float volume)
float ma_engine_get_volume(ma_engine* pEngine)
ma_result ma_engine_set_gain_db(ma_engine* pEngine, float gainDB)
float ma_engine_get_gain_db(ma_engine* pEngine)

ma_uint32 ma_engine_get_listener_count(const ma_engine* pEngine)
ma_uint32 ma_engine_find_closest_listener(const ma_engine* pEngine, float absolutePosX, float absolutePosY, float absolutePosZ)
void ma_engine_listener_set_position(ma_engine* pEngine, ma_uint32 listenerIndex, float x, float y, float z)
ma_vec3f ma_engine_listener_get_position(const ma_engine* pEngine, ma_uint32 listenerIndex)
void ma_engine_listener_set_direction(ma_engine* pEngine, ma_uint32 listenerIndex, float x, float y, float z)
ma_vec3f ma_engine_listener_get_direction(const ma_engine* pEngine, ma_uint32 listenerIndex)
void ma_engine_listener_set_velocity(ma_engine* pEngine, ma_uint32 listenerIndex, float x, float y, float z)
ma_vec3f ma_engine_listener_get_velocity(const ma_engine* pEngine, ma_uint32 listenerIndex)
void ma_engine_listener_set_cone(ma_engine* pEngine, ma_uint32 listenerIndex, float innerAngleInRadians, float outerAngleInRadians, float outerGain)
void ma_engine_listener_get_cone(const ma_engine* pEngine, ma_uint32 listenerIndex, float* pInnerAngleInRadians, float* pOuterAngleInRadians, float* pOuterGain)
void ma_engine_listener_set_world_up(ma_engine* pEngine, ma_uint32 listenerIndex, float x, float y, float z)
ma_vec3f ma_engine_listener_get_world_up(const ma_engine* pEngine, ma_uint32 listenerIndex)
void ma_engine_listener_set_enabled(ma_engine* pEngine, ma_uint32 listenerIndex, ma_bool32 isEnabled)
ma_bool32 ma_engine_listener_is_enabled(const ma_engine* pEngine, ma_uint32 listenerIndex)


ma_result ma_engine_play_sound_ex(ma_engine* pEngine, const char* pFilePath, ma_node* pNode, ma_uint32 nodeInputBusIndex)
ma_result ma_engine_play_sound(ma_engine* pEngine, const char* pFilePath, ma_sound_group* pGroup)



ma_result ma_sound_init_from_file(ma_engine* pEngine, const char* pFilePath, ma_uint32 flags, ma_sound_group* pGroup, ma_fence* pDoneFence, ma_sound* pSound)
ma_result ma_sound_init_from_file_w(ma_engine* pEngine, const wchar_t* pFilePath, ma_uint32 flags, ma_sound_group* pGroup, ma_fence* pDoneFence, ma_sound* pSound)
ma_result ma_sound_init_copy(ma_engine* pEngine, const ma_sound* pExistingSound, ma_uint32 flags, ma_sound_group* pGroup, ma_sound* pSound)

ma_result ma_sound_init_from_data_source(ma_engine* pEngine, ma_data_source* pDataSource, ma_uint32 flags, ma_sound_group* pGroup, ma_sound* pSound)
ma_result ma_sound_init_ex(ma_engine* pEngine, const ma_sound_config* pConfig, ma_sound* pSound)
void ma_sound_uninit(ma_sound* pSound)
ma_engine* ma_sound_get_engine(const ma_sound* pSound)
ma_data_source* ma_sound_get_data_source(const ma_sound* pSound)
ma_result ma_sound_start(ma_sound* pSound)
ma_result ma_sound_stop(ma_sound* pSound)
ma_result ma_sound_stop_with_fade_in_pcm_frames(ma_sound* pSound, ma_uint64 fadeLengthInFrames)
ma_result ma_sound_stop_with_fade_in_milliseconds(ma_sound* pSound, ma_uint64 fadeLengthInFrames)
void ma_sound_set_volume(ma_sound* pSound, float volume)
float ma_sound_get_volume(const ma_sound* pSound)
void ma_sound_set_pan(ma_sound* pSound, float pan)
float ma_sound_get_pan(const ma_sound* pSound)
void ma_sound_set_pan_mode(ma_sound* pSound, ma_pan_mode panMode)
ma_pan_mode ma_sound_get_pan_mode(const ma_sound* pSound)
void ma_sound_set_pitch(ma_sound* pSound, float pitch)
float ma_sound_get_pitch(const ma_sound* pSound)
void ma_sound_set_spatialization_enabled(ma_sound* pSound, ma_bool32 enabled)
ma_bool32 ma_sound_is_spatialization_enabled(const ma_sound* pSound)
void ma_sound_set_pinned_listener_index(ma_sound* pSound, ma_uint32 listenerIndex)
ma_uint32 ma_sound_get_pinned_listener_index(const ma_sound* pSound)
ma_uint32 ma_sound_get_listener_index(const ma_sound* pSound)
ma_vec3f ma_sound_get_direction_to_listener(const ma_sound* pSound)
void ma_sound_set_position(ma_sound* pSound, float x, float y, float z)
ma_vec3f ma_sound_get_position(const ma_sound* pSound)
void ma_sound_set_direction(ma_sound* pSound, float x, float y, float z)
ma_vec3f ma_sound_get_direction(const ma_sound* pSound)
void ma_sound_set_velocity(ma_sound* pSound, float x, float y, float z)
ma_vec3f ma_sound_get_velocity(const ma_sound* pSound)
void ma_sound_set_attenuation_model(ma_sound* pSound, ma_attenuation_model attenuationModel)
ma_attenuation_model ma_sound_get_attenuation_model(const ma_sound* pSound)
void ma_sound_set_positioning(ma_sound* pSound, ma_positioning positioning)
ma_positioning ma_sound_get_positioning(const ma_sound* pSound)
void ma_sound_set_rolloff(ma_sound* pSound, float rolloff)
float ma_sound_get_rolloff(const ma_sound* pSound)
void ma_sound_set_min_gain(ma_sound* pSound, float minGain)
float ma_sound_get_min_gain(const ma_sound* pSound)
void ma_sound_set_max_gain(ma_sound* pSound, float maxGain)
float ma_sound_get_max_gain(const ma_sound* pSound)
void ma_sound_set_min_distance(ma_sound* pSound, float minDistance)
float ma_sound_get_min_distance(const ma_sound* pSound)
void ma_sound_set_max_distance(ma_sound* pSound, float maxDistance)
float ma_sound_get_max_distance(const ma_sound* pSound)
void ma_sound_set_cone(ma_sound* pSound, float innerAngleInRadians, float outerAngleInRadians, float outerGain)
void ma_sound_get_cone(const ma_sound* pSound, float* pInnerAngleInRadians, float* pOuterAngleInRadians, float* pOuterGain)
void ma_sound_set_doppler_factor(ma_sound* pSound, float dopplerFactor)
float ma_sound_get_doppler_factor(const ma_sound* pSound)
void ma_sound_set_directional_attenuation_factor(ma_sound* pSound, float directionalAttenuationFactor)
float ma_sound_get_directional_attenuation_factor(const ma_sound* pSound)
void ma_sound_set_fade_in_pcm_frames(ma_sound* pSound, float volumeBeg, float volumeEnd, ma_uint64 fadeLengthInFrames)
void ma_sound_set_fade_in_milliseconds(ma_sound* pSound, float volumeBeg, float volumeEnd, ma_uint64 fadeLengthInMilliseconds)
void ma_sound_set_fade_start_in_pcm_frames(ma_sound* pSound, float volumeBeg, float volumeEnd, ma_uint64 fadeLengthInFrames, ma_uint64 absoluteGlobalTimeInFrames)
void ma_sound_set_fade_start_in_milliseconds(ma_sound* pSound, float volumeBeg, float volumeEnd, ma_uint64 fadeLengthInMilliseconds, ma_uint64 absoluteGlobalTimeInMilliseconds)
float ma_sound_get_current_fade_volume(const ma_sound* pSound)
void ma_sound_set_start_time_in_pcm_frames(ma_sound* pSound, ma_uint64 absoluteGlobalTimeInFrames)
void ma_sound_set_start_time_in_milliseconds(ma_sound* pSound, ma_uint64 absoluteGlobalTimeInMilliseconds)
void ma_sound_set_stop_time_in_pcm_frames(ma_sound* pSound, ma_uint64 absoluteGlobalTimeInFrames)
void ma_sound_set_stop_time_in_milliseconds(ma_sound* pSound, ma_uint64 absoluteGlobalTimeInMilliseconds)
void ma_sound_set_stop_time_with_fade_in_pcm_frames(ma_sound* pSound, ma_uint64 stopAbsoluteGlobalTimeInFrames, ma_uint64 fadeLengthInFrames)
void ma_sound_set_stop_time_with_fade_in_milliseconds(ma_sound* pSound, ma_uint64 stopAbsoluteGlobalTimeInMilliseconds, ma_uint64 fadeLengthInMilliseconds)
ma_bool32 ma_sound_is_playing(const ma_sound* pSound)
ma_uint64 ma_sound_get_time_in_pcm_frames(const ma_sound* pSound)
ma_uint64 ma_sound_get_time_in_milliseconds(const ma_sound* pSound)
void ma_sound_set_looping(ma_sound* pSound, ma_bool32 isLooping)
ma_bool32 ma_sound_is_looping(const ma_sound* pSound)
ma_bool32 ma_sound_at_end(const ma_sound* pSound)
ma_result ma_sound_seek_to_pcm_frame(ma_sound* pSound, ma_uint64 frameIndex)
ma_result ma_sound_get_data_format(ma_sound* pSound, ma_format* pFormat, ma_uint32* pChannels, ma_uint32* pSampleRate, ma_channel* pChannelMap, size_t channelMapCap)
ma_result ma_sound_get_cursor_in_pcm_frames(ma_sound* pSound, ma_uint64* pCursor)
ma_result ma_sound_get_length_in_pcm_frames(ma_sound* pSound, ma_uint64* pLength)
ma_result ma_sound_get_cursor_in_seconds(ma_sound* pSound, float* pCursor)
ma_result ma_sound_get_length_in_seconds(ma_sound* pSound, float* pLength)
ma_result ma_sound_set_end_callback(ma_sound* pSound, ma_sound_end_proc callback, void* pUserData)

ma_result ma_sound_group_init(ma_engine* pEngine, ma_uint32 flags, ma_sound_group* pParentGroup, ma_sound_group* pGroup)
ma_result ma_sound_group_init_ex(ma_engine* pEngine, const ma_sound_group_config* pConfig, ma_sound_group* pGroup)
void ma_sound_group_uninit(ma_sound_group* pGroup)
ma_engine* ma_sound_group_get_engine(const ma_sound_group* pGroup)
ma_result ma_sound_group_start(ma_sound_group* pGroup)
ma_result ma_sound_group_stop(ma_sound_group* pGroup)
void ma_sound_group_set_volume(ma_sound_group* pGroup, float volume)
float ma_sound_group_get_volume(const ma_sound_group* pGroup)
void ma_sound_group_set_pan(ma_sound_group* pGroup, float pan)
float ma_sound_group_get_pan(const ma_sound_group* pGroup)
void ma_sound_group_set_pan_mode(ma_sound_group* pGroup, ma_pan_mode panMode)
ma_pan_mode ma_sound_group_get_pan_mode(const ma_sound_group* pGroup)
void ma_sound_group_set_pitch(ma_sound_group* pGroup, float pitch)
float ma_sound_group_get_pitch(const ma_sound_group* pGroup)
void ma_sound_group_set_spatialization_enabled(ma_sound_group* pGroup, ma_bool32 enabled)
ma_bool32 ma_sound_group_is_spatialization_enabled(const ma_sound_group* pGroup)
void ma_sound_group_set_pinned_listener_index(ma_sound_group* pGroup, ma_uint32 listenerIndex)
ma_uint32 ma_sound_group_get_pinned_listener_index(const ma_sound_group* pGroup)
ma_uint32 ma_sound_group_get_listener_index(const ma_sound_group* pGroup)
ma_vec3f ma_sound_group_get_direction_to_listener(const ma_sound_group* pGroup)
void ma_sound_group_set_position(ma_sound_group* pGroup, float x, float y, float z)
ma_vec3f ma_sound_group_get_position(const ma_sound_group* pGroup)
void ma_sound_group_set_direction(ma_sound_group* pGroup, float x, float y, float z)
ma_vec3f ma_sound_group_get_direction(const ma_sound_group* pGroup)
void ma_sound_group_set_velocity(ma_sound_group* pGroup, float x, float y, float z)
ma_vec3f ma_sound_group_get_velocity(const ma_sound_group* pGroup)
void ma_sound_group_set_attenuation_model(ma_sound_group* pGroup, ma_attenuation_model attenuationModel)
ma_attenuation_model ma_sound_group_get_attenuation_model(const ma_sound_group* pGroup)
void ma_sound_group_set_positioning(ma_sound_group* pGroup, ma_positioning positioning)
ma_positioning ma_sound_group_get_positioning(const ma_sound_group* pGroup)
void ma_sound_group_set_rolloff(ma_sound_group* pGroup, float rolloff)
float ma_sound_group_get_rolloff(const ma_sound_group* pGroup)
void ma_sound_group_set_min_gain(ma_sound_group* pGroup, float minGain)
float ma_sound_group_get_min_gain(const ma_sound_group* pGroup)
void ma_sound_group_set_max_gain(ma_sound_group* pGroup, float maxGain)
float ma_sound_group_get_max_gain(const ma_sound_group* pGroup)
void ma_sound_group_set_min_distance(ma_sound_group* pGroup, float minDistance)
float ma_sound_group_get_min_distance(const ma_sound_group* pGroup)
void ma_sound_group_set_max_distance(ma_sound_group* pGroup, float maxDistance)
float ma_sound_group_get_max_distance(const ma_sound_group* pGroup)
void ma_sound_group_set_cone(ma_sound_group* pGroup, float innerAngleInRadians, float outerAngleInRadians, float outerGain)
void ma_sound_group_get_cone(const ma_sound_group* pGroup, float* pInnerAngleInRadians, float* pOuterAngleInRadians, float* pOuterGain)
void ma_sound_group_set_doppler_factor(ma_sound_group* pGroup, float dopplerFactor)
float ma_sound_group_get_doppler_factor(const ma_sound_group* pGroup)
void ma_sound_group_set_directional_attenuation_factor(ma_sound_group* pGroup, float directionalAttenuationFactor)
float ma_sound_group_get_directional_attenuation_factor(const ma_sound_group* pGroup)
void ma_sound_group_set_fade_in_pcm_frames(ma_sound_group* pGroup, float volumeBeg, float volumeEnd, ma_uint64 fadeLengthInFrames)
void ma_sound_group_set_fade_in_milliseconds(ma_sound_group* pGroup, float volumeBeg, float volumeEnd, ma_uint64 fadeLengthInMilliseconds)
float ma_sound_group_get_current_fade_volume(ma_sound_group* pGroup)
void ma_sound_group_set_start_time_in_pcm_frames(ma_sound_group* pGroup, ma_uint64 absoluteGlobalTimeInFrames)
void ma_sound_group_set_start_time_in_milliseconds(ma_sound_group* pGroup, ma_uint64 absoluteGlobalTimeInMilliseconds)
void ma_sound_group_set_stop_time_in_pcm_frames(ma_sound_group* pGroup, ma_uint64 absoluteGlobalTimeInFrames)
void ma_sound_group_set_stop_time_in_milliseconds(ma_sound_group* pGroup, ma_uint64 absoluteGlobalTimeInMilliseconds)
ma_bool32 ma_sound_group_is_playing(const ma_sound_group* pGroup)
ma_uint64 ma_sound_group_get_time_in_pcm_frames(const ma_sound_group* pGroup)


