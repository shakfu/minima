

cdef extern from "miniaudio.h":
    """
    #define MA_NO_RUNTIME_LINKING
    #define MA_ENABLE_ONLY_SPECIFIC_BACKENDS
    #define MA_ENABLE_COREAUDIO
    #define MINIAUDIO_IMPLEMENTATION
    """
    ctypedef enum ma_device_type:
        ma_device_type_playback = 1
        ma_device_type_capture  = 2
        ma_device_type_playback = 3
        ma_device_type_loopback = 4


    ctypedef enum ma_stream_format:
        ma_stream_format_pcm = 0

    ctypedef enum ma_stream_layout:
        ma_stream_layout_interleaved   = 0
        ma_stream_layout_deinterleaved = 1

    ctypedef enum ma_dither_mode: 
        ma_dither_mode_none      = 0
        ma_dither_mode_rectangle = 1
        ma_dither_mode_triangle  = 2


    ctypedef enum ma_format:
        ma_format_unknown = 0
        ma_format_u8      = 1
        ma_format_s16     = 2
        ma_format_s24     = 3
        ma_format_s32     = 4
        ma_format_f32     = 5
        ma_format_count   = 6

    ctypedef enum ma_standard_sample_rate:

        ma_standard_sample_rate_48000  = 48000
        ma_standard_sample_rate_44100  = 44100

        ma_standard_sample_rate_32000  = 32000
        ma_standard_sample_rate_24000  = 24000
        ma_standard_sample_rate_22050  = 22050

        ma_standard_sample_rate_88200  = 88200
        ma_standard_sample_rate_96000  = 96000
        ma_standard_sample_rate_176400 = 176400
        ma_standard_sample_rate_192000 = 192000

        ma_standard_sample_rate_16000  = 16000
        ma_standard_sample_rate_11025  = 11250
        ma_standard_sample_rate_8000   = 8000

        ma_standard_sample_rate_352800 = 352800
        ma_standard_sample_rate_384000 = 384000

        ma_standard_sample_rate_min    = ma_standard_sample_rate_8000,
        ma_standard_sample_rate_max    = ma_standard_sample_rate_384000,
        ma_standard_sample_rate_count  = 14


    # ctypedef struct ma_context_config    ma_context_config
    # ctypedef struct ma_device_config     ma_device_config
    # ctypedef struct ma_backend_callbacks ma_backend_callbacks

    ctypedef   signed char           ma_int8
    ctypedef unsigned char           ma_uint8
    ctypedef   signed short          ma_int16
    ctypedef unsigned short          ma_uint16
    ctypedef   signed int            ma_int32
    ctypedef unsigned int            ma_uint32

    ctypedef ma_uint8    ma_bool8
    ctypedef ma_uint32   ma_bool32


    cdef struct linear:
        ma_uint32 lpfOrder

    cdef struct speex:
        int quality

    cdef struct resampling:
        # ma_resample_algorithm algorithm
        linear linear
        speex speex

    cdef struct coreaudio:
        ma_bool32 allowNominalSampleRateChange


    # cdef struct playback:
    #     ma_device_id* pDeviceID
    #     ma_format format
    #     ma_uint32 channels
    #     ma_channel channelMap[MA_MAX_CHANNELS]
    #     ma_channel_mix_mode channelMixMode
    #     ma_share_mode shareMode

    # cdef struct capture:
    #     ma_device_id* pDeviceID
    #     ma_format format
    #     ma_uint32 channels
    #     ma_channel channelMap[MA_MAX_CHANNELS]
    #     ma_channel_mix_mode channelMixMode
    #     ma_share_mode shareMode

    ctypedef struct ma_device_config:

        ma_device_type deviceType
        ma_uint32 sampleRate
        ma_uint32 periodSizeInFrames
        ma_uint32 periodSizeInMilliseconds
        ma_uint32 periods
        # ma_performance_profile performanceProfile
        ma_bool8 noPreZeroedOutputBuffer  
        ma_bool8 noClip
        # ma_device_callback_proc dataCallback
    #     ma_stop_proc stopCallback

        void* pUserData

    # miniaudio api functions
    # void ma_version(ma_uint32* pMajor, ma_uint32* pMinor, ma_uint32* pRevision)
    char* ma_version_string()



