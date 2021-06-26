

cdef extern from "<pthread.h>" nogil:
    # see: https://github.com/python-llfuse/python-llfuse/blob/master/Include/pthread.pxd

    ctypedef unsigned long int pthread_t

    ctypedef union pthread_mutex_t:
        pass

    ctypedef union pthread_cond_t:
        pass



cdef extern from "miniaudio.h":
    """
    #define MA_NO_RUNTIME_LINKING
    #define MA_ENABLE_ONLY_SPECIFIC_BACKENDS
    #define MA_ENABLE_COREAUDIO
    #define MINIAUDIO_IMPLEMENTATION
    """

    DEF MA_MAX_CHANNELS = 32

    ctypedef   signed char      ma_int8
    ctypedef unsigned char      ma_uint8
    ctypedef   signed short     ma_int16
    ctypedef unsigned short     ma_uint16
    ctypedef   signed int       ma_int32
    ctypedef unsigned int       ma_uint32
    ctypedef signed long long   ma_int64
    ctypedef unsigned long long ma_uint64

    ctypedef ma_uint64 ma_uintptr
    ctypedef ma_uint8  ma_bool8
    ctypedef ma_uint32 ma_bool32

    ctypedef void *ma_handle
    ctypedef void *ma_ptr
    ctypedef void (*ma_proc)()

    ctypedef struct ma_context
    ctypedef struct ma_device

    ctypedef ma_uint8 ma_channel
    ctypedef int ma_result

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

    ctypedef enum ma_channel_mix_mode:
        ma_channel_mix_mode_rectangular = 0
        ma_channel_mix_mode_simple
        ma_channel_mix_mode_custom_weights
        ma_channel_mix_mode_planar_blend = ma_channel_mix_mode_rectangular
        ma_channel_mix_mode_default = ma_channel_mix_mode_planar_blend

    ctypedef enum ma_standard_channel_map:
        ma_standard_channel_map_microsoft
        ma_standard_channel_map_alsa
        ma_standard_channel_map_rfc3551
        ma_standard_channel_map_flac
        ma_standard_channel_map_vorbis
        ma_standard_channel_map_sound4
        ma_standard_channel_map_sndio
        ma_standard_channel_map_webaudio = ma_standard_channel_map_flac
        ma_standard_channel_map_default = ma_standard_channel_map_microsoft

    ctypedef enum ma_performance_profile:
        ma_performance_profile_low_latency = 0
        ma_performance_profile_conservative


    ctypedef struct ma_allocation_callbacks:
        void *pUserData
        void *(*onMalloc)(size_t sz, void *pUserData)
        void *(*onRealloc)(void *p, size_t sz, void *pUserData)
        void (*onFree)(void *p, void *pUserData)

    ctypedef struct ma_lcg:
        ma_int32 state

    ctypedef enum ma_thread_priority:
        ma_thread_priority_idle = -5
        ma_thread_priority_lowest = -4
        ma_thread_priority_low = -3
        ma_thread_priority_normal = -2
        ma_thread_priority_high = -1
        ma_thread_priority_highest = 0
        ma_thread_priority_realtime = 1
        ma_thread_priority_default = 0

    ctypedef ma_uint32 ma_spinlock

    ctypedef pthread_t ma_thread

    ctypedef pthread_mutex_t ma_mutex

    ctypedef struct ma_event:
        ma_uint32 value
        pthread_mutex_t lock
        pthread_cond_t cond

    ctypedef struct ma_semaphore:
        int value
        pthread_mutex_t lock
        pthread_cond_t cond

    void ma_version(ma_uint32* pMajor, ma_uint32* pMinor, ma_uint32* pRevision)
    char* ma_version_string()

    ctypedef union ma_biquad_coefficient:
        float f32
        ma_int32 s32

    ctypedef struct ma_biquad_config:
        ma_format format
        ma_uint32 channels
        double b0
        double b1
        double b2
        double a0
        double a1
        double a2

    # API
    ma_biquad_config ma_biquad_config_init(
        ma_format format,
        ma_uint32 channels, double b0,
        double b1, double b2, double a0,
        double a1, double a2)

    ctypedef struct ma_biquad:
        ma_format format
        ma_uint32 channels
        ma_biquad_coefficient b0
        ma_biquad_coefficient b1
        ma_biquad_coefficient b2
        ma_biquad_coefficient a1
        ma_biquad_coefficient a2
        ma_biquad_coefficient r1[32]
        ma_biquad_coefficient r2[32]

    # API
    ma_result ma_biquad_init(const ma_biquad_config *pConfig, ma_biquad *pBQ)
    ma_result ma_biquad_reinit(const ma_biquad_config *pConfig, ma_biquad *pBQ)
    ma_result ma_biquad_process_pcm_frames(ma_biquad *pBQ, void *pFramesOut,
                                           const void *pFramesIn, ma_uint64 frameCount)
    ma_uint32 ma_biquad_get_latency(const ma_biquad *pBQ)


    ctypedef struct ma_lpf1_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double cutoffFrequency
        double q

    ctypedef ma_lpf1_config ma_lpf2_config


    ma_lpf1_config ma_lpf1_config_init(ma_format format, ma_uint32 channels,
                                              ma_uint32 sampleRate,
                                              double cutoffFrequency)
    ma_lpf2_config ma_lpf2_config_init(ma_format format, ma_uint32 channels,
                                              ma_uint32 sampleRate,
                                              double cutoffFrequency, double q)


    ctypedef struct ma_lpf1:
        ma_format format
        ma_uint32 channels
        ma_biquad_coefficient a
        ma_biquad_coefficient r1[32]
    

    ma_result ma_lpf1_init(const ma_lpf1_config *pConfig, ma_lpf1 *pLPF)
    ma_result ma_lpf1_reinit(const ma_lpf1_config *pConfig, ma_lpf1 *pLPF)
    ma_result ma_lpf1_process_pcm_frames(ma_lpf1 *pLPF, void *pFramesOut,
                                                const void *pFramesIn,
                                                ma_uint64 frameCount)
    ma_uint32 ma_lpf1_get_latency(const ma_lpf1 *pLPF)

    ctypedef struct ma_lpf2:
        ma_biquad bq

    ma_result ma_lpf2_init(const ma_lpf2_config *pConfig, ma_lpf2 *pLPF)
    ma_result ma_lpf2_reinit(const ma_lpf2_config *pConfig, ma_lpf2 *pLPF)
    ma_result ma_lpf2_process_pcm_frames(ma_lpf2 *pLPF, void *pFramesOut,
                                                const void *pFramesIn,
                                                ma_uint64 frameCount)
    ma_uint32 ma_lpf2_get_latency(const ma_lpf2 *pLPF)

    ctypedef struct ma_lpf_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double cutoffFrequency
        ma_uint32 order



    ma_lpf_config ma_lpf_config_init(ma_format format, ma_uint32 channels,
                                            ma_uint32 sampleRate,
                                            double cutoffFrequency,
                                            ma_uint32 order)

    ctypedef struct ma_lpf:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        ma_uint32 lpf1Count
        ma_uint32 lpf2Count
        ma_lpf1 lpf1[1]
        ma_lpf2 lpf2[4] # was lfp2[8 / 2]


    ma_result ma_lpf_init(const ma_lpf_config *pConfig, ma_lpf *pLPF)
    ma_result ma_lpf_reinit(const ma_lpf_config *pConfig, ma_lpf *pLPF)
    ma_result ma_lpf_process_pcm_frames(ma_lpf *pLPF, void *pFramesOut,
                                               const void *pFramesIn,
                                               ma_uint64 frameCount)
    ma_uint32 ma_lpf_get_latency(const ma_lpf *pLPF)

    ctypedef struct ma_hpf1_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double cutoffFrequency
        double q

    ctypedef ma_hpf1_config ma_hpf2_config

    ma_hpf1_config ma_hpf1_config_init(ma_format format, ma_uint32 channels,
                                              ma_uint32 sampleRate,
                                              double cutoffFrequency)
    ma_hpf2_config ma_hpf2_config_init(ma_format format, ma_uint32 channels,
                                              ma_uint32 sampleRate,
                                              double cutoffFrequency, double q)

    ctypedef struct ma_hpf1:
        ma_format format
        ma_uint32 channels
        ma_biquad_coefficient a
        ma_biquad_coefficient r1[32]



    ma_result ma_hpf1_init(const ma_hpf1_config *pConfig, ma_hpf1 *pHPF)
    ma_result ma_hpf1_reinit(const ma_hpf1_config *pConfig, ma_hpf1 *pHPF)
    ma_result ma_hpf1_process_pcm_frames(ma_hpf1 *pHPF, void *pFramesOut,
                                                const void *pFramesIn,
                                                ma_uint64 frameCount)
    ma_uint32 ma_hpf1_get_latency(const ma_hpf1 *pHPF)

    ctypedef struct ma_hpf2:
        ma_biquad bq


    ma_result ma_hpf2_init(const ma_hpf2_config *pConfig, ma_hpf2 *pHPF)
    ma_result ma_hpf2_reinit(const ma_hpf2_config *pConfig, ma_hpf2 *pHPF)
    ma_result ma_hpf2_process_pcm_frames(ma_hpf2 *pHPF, void *pFramesOut,
                                                const void *pFramesIn,
                                                ma_uint64 frameCount)
    ma_uint32 ma_hpf2_get_latency(const ma_hpf2 *pHPF)

    ctypedef struct ma_hpf_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double cutoffFrequency
        ma_uint32 order

    ma_hpf_config ma_hpf_config_init(ma_format format, ma_uint32 channels,
                                            ma_uint32 sampleRate,
                                            double cutoffFrequency,
                                            ma_uint32 order)

    ctypedef struct ma_hpf:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        ma_uint32 hpf1Count
        ma_uint32 hpf2Count
        ma_hpf1 hpf1[1]
        ma_hpf2 hpf2[4] # was hpf2[8 / 2]




    ma_result ma_hpf_init(const ma_hpf_config *pConfig, ma_hpf *pHPF)
    ma_result ma_hpf_reinit(const ma_hpf_config *pConfig, ma_hpf *pHPF)
    ma_result ma_hpf_process_pcm_frames(ma_hpf *pHPF, void *pFramesOut,
                                               const void *pFramesIn,
                                               ma_uint64 frameCount)
    ma_uint32 ma_hpf_get_latency(const ma_hpf *pHPF)

    ctypedef struct ma_bpf2_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double cutoffFrequency
        double q


    ma_bpf2_config ma_bpf2_config_init(ma_format format, ma_uint32 channels,
                                              ma_uint32 sampleRate,
                                              double cutoffFrequency, double q)

    ctypedef struct ma_bpf2:
        ma_biquad bq

    ma_result ma_bpf2_init(const ma_bpf2_config *pConfig, ma_bpf2 *pBPF)
    ma_result ma_bpf2_reinit(const ma_bpf2_config *pConfig, ma_bpf2 *pBPF)
    ma_result ma_bpf2_process_pcm_frames(ma_bpf2 *pBPF, void *pFramesOut,
                                                const void *pFramesIn,
                                                ma_uint64 frameCount)
    ma_uint32 ma_bpf2_get_latency(const ma_bpf2 *pBPF)

    ctypedef struct ma_bpf_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double cutoffFrequency
        ma_uint32 order

    ma_bpf_config ma_bpf_config_init(ma_format format, ma_uint32 channels,
                                            ma_uint32 sampleRate,
                                            double cutoffFrequency,
                                            ma_uint32 order)

    ctypedef struct ma_bpf:
        ma_format format
        ma_uint32 channels
        ma_uint32 bpf2Count
        ma_bpf2 bpf2[4] # was bpf2[8 / 2]

    ma_result ma_bpf_init(const ma_bpf_config *pConfig, ma_bpf *pBPF)
    ma_result ma_bpf_reinit(const ma_bpf_config *pConfig, ma_bpf *pBPF)
    ma_result ma_bpf_process_pcm_frames(ma_bpf *pBPF, void *pFramesOut,
                                               const void *pFramesIn,
                                               ma_uint64 frameCount)
    ma_uint32 ma_bpf_get_latency(const ma_bpf *pBPF)

    ctypedef struct ma_notch2_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double q
        double frequency

    ma_notch2_config ma_notch2_config_init(ma_format format,
                                                  ma_uint32 channels,
                                                  ma_uint32 sampleRate, double q,
                                                  double frequency)

    ctypedef struct ma_notch2:
        ma_biquad bq

    ma_result ma_notch2_init(const ma_notch2_config *pConfig,
                                    ma_notch2 *pFilter)
    ma_result ma_notch2_reinit(const ma_notch2_config *pConfig,
                                      ma_notch2 *pFilter)
    ma_result ma_notch2_process_pcm_frames(ma_notch2 *pFilter,
                                                  void *pFramesOut,
                                                  const void *pFramesIn,
                                                  ma_uint64 frameCount)
    ma_uint32 ma_notch2_get_latency(const ma_notch2 *pFilter)




    ctypedef struct ma_peak2_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double gainDB
        double q
        double frequency

    ma_peak2_config ma_peak2_config_init(ma_format format,
                                                ma_uint32 channels,
                                                ma_uint32 sampleRate, double gainDB,
                                                double q, double frequency)

    ctypedef struct ma_peak2:
        ma_biquad bq


    ma_result ma_peak2_init(const ma_peak2_config *pConfig,
                                   ma_peak2 *pFilter)
    ma_result ma_peak2_reinit(const ma_peak2_config *pConfig,
                                     ma_peak2 *pFilter)
    ma_result ma_peak2_process_pcm_frames(ma_peak2 *pFilter,
                                                 void *pFramesOut,
                                                 const void *pFramesIn,
                                                 ma_uint64 frameCount)
    ma_uint32 ma_peak2_get_latency(const ma_peak2 *pFilter)

    ctypedef struct ma_loshelf2_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double gainDB
        double shelfSlope
        double frequency


    ma_loshelf2_config ma_loshelf2_config_init(ma_format format, ma_uint32 channels,
                            ma_uint32 sampleRate, double gainDB, double shelfSlope,
                            double frequency)

    ctypedef struct ma_loshelf2:
        ma_biquad bq


    ma_result ma_loshelf2_init(const ma_loshelf2_config *pConfig,
                                      ma_loshelf2 *pFilter)
    ma_result ma_loshelf2_reinit(const ma_loshelf2_config *pConfig,
                                        ma_loshelf2 *pFilter)
    ma_result ma_loshelf2_process_pcm_frames(ma_loshelf2 *pFilter,
                                                    void *pFramesOut,
                                                    const void *pFramesIn,
                                                    ma_uint64 frameCount)
    ma_uint32 ma_loshelf2_get_latency(const ma_loshelf2 *pFilter)

    ctypedef struct ma_hishelf2_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        double gainDB
        double shelfSlope
        double frequency

    ma_hishelf2_config ma_hishelf2_config_init(ma_format format, ma_uint32 channels,
                            ma_uint32 sampleRate, double gainDB, double shelfSlope,
                            double frequency)

    ctypedef struct ma_hishelf2:
        ma_biquad bq


    ma_result ma_hishelf2_init(const ma_hishelf2_config *pConfig,
                                      ma_hishelf2 *pFilter)
    ma_result ma_hishelf2_reinit(const ma_hishelf2_config *pConfig,
                                        ma_hishelf2 *pFilter)
    ma_result ma_hishelf2_process_pcm_frames(ma_hishelf2 *pFilter,
                                                    void *pFramesOut,
                                                    const void *pFramesIn,
                                                    ma_uint64 frameCount)
    ma_uint32 ma_hishelf2_get_latency(const ma_hishelf2 *pFilter)

    ctypedef struct ma_linear_resampler_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRateIn
        ma_uint32 sampleRateOut
        ma_uint32 lpfOrder
        double lpfNyquistFactor

    ma_linear_resampler_config ma_linear_resampler_config_init(
        ma_format format, ma_uint32 channels,
        ma_uint32 sampleRateIn,
        ma_uint32 sampleRateOut)


    cdef union ma_linear_resampler__x0:
        float f32[32]
        ma_int16 s16[32]
    cdef union ma_linear_resampler__x1:
        float f32[32]
        ma_int16 s16[32]
    ctypedef struct ma_linear_resampler:
        ma_linear_resampler_config config
        ma_uint32 inAdvanceInt
        ma_uint32 inAdvanceFrac
        ma_uint32 inTimeInt
        ma_uint32 inTimeFrac
        ma_linear_resampler__x0 x0
        ma_linear_resampler__x1 x1
        ma_lpf lpf

    ma_result ma_linear_resampler_init(const ma_linear_resampler_config *pConfig,
                             ma_linear_resampler *pResampler)
    void ma_linear_resampler_uninit(ma_linear_resampler *pResampler)
    ma_result ma_linear_resampler_process_pcm_frames(
        ma_linear_resampler *pResampler, const void *pFramesIn,
        ma_uint64 *pFrameCountIn, void *pFramesOut, ma_uint64 *pFrameCountOut)
    ma_result ma_linear_resampler_set_rate(ma_linear_resampler *pResampler,
                                                  ma_uint32 sampleRateIn,
                                                  ma_uint32 sampleRateOut)
    ma_result ma_linear_resampler_set_rate_ratio(ma_linear_resampler *pResampler,
                                       float ratioInOut)
    ma_uint64 ma_linear_resampler_get_required_input_frame_count(
        const ma_linear_resampler *pResampler, ma_uint64 outputFrameCount)
    ma_uint64 ma_linear_resampler_get_expected_output_frame_count(
        const ma_linear_resampler *pResampler, ma_uint64 inputFrameCount)
    ma_uint64 ma_linear_resampler_get_input_latency(const ma_linear_resampler *pResampler)
    ma_uint64 ma_linear_resampler_get_output_latency(const ma_linear_resampler *pResampler)

    ctypedef enum ma_resample_algorithm:
        ma_resample_algorithm_linear = 0
        ma_resample_algorithm_speex


    cdef struct ma_resampler_config__linear:
        ma_uint32 lpfOrder
        double lpfNyquistFactor
    cdef struct ma_resampler_config__speex:
        int quality
    ctypedef struct ma_resampler_config:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRateIn
        ma_uint32 sampleRateOut
        ma_resample_algorithm algorithm
        ma_resampler_config__linear linear
        ma_resampler_config__speex speex


    ma_resampler_config ma_resampler_config_init(ma_format format, ma_uint32 channels,
                             ma_uint32 sampleRateIn, ma_uint32 sampleRateOut,
                             ma_resample_algorithm algorithm)



    cdef struct ma_resampler__state__speex:
        void *pSpeexResamplerState

    cdef union ma_resampler__state:
        ma_linear_resampler linear
        ma_resampler__state__speex speex

    ctypedef struct ma_resampler:
        ma_resampler_config config
        ma_resampler__state state


    ma_result ma_resampler_init(const ma_resampler_config *pConfig,
                                       ma_resampler *pResampler)

    void ma_resampler_uninit(ma_resampler *pResampler)

    ma_result ma_resampler_process_pcm_frames(ma_resampler *pResampler,
                                                     const void *pFramesIn,
                                                     ma_uint64 *pFrameCountIn,
                                                     void *pFramesOut,
                                                     ma_uint64 *pFrameCountOut)

    ma_result ma_resampler_set_rate(ma_resampler *pResampler,
                                           ma_uint32 sampleRateIn,
                                           ma_uint32 sampleRateOut)

    ma_result ma_resampler_set_rate_ratio(ma_resampler *pResampler,
                                                 float ratio)

    ma_uint64 ma_resampler_get_required_input_frame_count(const ma_resampler *pResampler,
                                                ma_uint64 outputFrameCount)

    ma_uint64 ma_resampler_get_expected_output_frame_count(const ma_resampler *pResampler,
                                                 ma_uint64 inputFrameCount)

    ma_uint64 ma_resampler_get_input_latency(const ma_resampler *pResampler)

    ma_uint64 ma_resampler_get_output_latency(const ma_resampler *pResampler)


    ctypedef struct ma_channel_converter_config:
        ma_format format
        ma_uint32 channelsIn
        ma_uint32 channelsOut
        ma_channel channelMapIn[32]
        ma_channel channelMapOut[32]
        ma_channel_mix_mode mixingMode
        float weights[32][32]


    ma_channel_converter_config ma_channel_converter_config_init(
        ma_format format, ma_uint32 channelsIn, const ma_channel *pChannelMapIn,
        ma_uint32 channelsOut, const ma_channel *pChannelMapOut,
        ma_channel_mix_mode mixingMode)


    cdef union ma_channel_converter__weights:
        float f32[32][32]
        ma_int32 s16[32][32]

    ctypedef struct ma_channel_converter:
        ma_format format
        ma_uint32 channelsIn
        ma_uint32 channelsOut
        ma_channel channelMapIn[32]
        ma_channel channelMapOut[32]
        ma_channel_mix_mode mixingMode
        ma_channel_converter__weights weights
        ma_bool8 isPassthrough
        ma_bool8 isSimpleShuffle
        ma_bool8 isSimpleMonoExpansion
        ma_bool8 isStereoToMono
        ma_uint8 shuffleTable[32]


    ma_result ma_channel_converter_init(const ma_channel_converter_config *pConfig,
                              ma_channel_converter *pConverter)
    void ma_channel_converter_uninit(ma_channel_converter *pConverter)
    ma_result ma_channel_converter_process_pcm_frames(ma_channel_converter *pConverter,
                                            void *pFramesOut, const void *pFramesIn,
                                            ma_uint64 frameCount)


    cdef struct ma_data_converter_config__resampling__linear:
        ma_uint32 lpfOrder
        double lpfNyquistFactor

    cdef struct ma_data_converter_config__resampling__speex:
        int quality

    cdef struct ma_data_converter_config__resampling:
        ma_resample_algorithm algorithm
        ma_bool32 allowDynamicSampleRate
        ma_data_converter_config__resampling__linear linear
        ma_data_converter_config__resampling__speex speex

    ctypedef struct ma_data_converter_config:
        ma_format formatIn
        ma_format formatOut
        ma_uint32 channelsIn
        ma_uint32 channelsOut
        ma_uint32 sampleRateIn
        ma_uint32 sampleRateOut
        ma_channel channelMapIn[32]
        ma_channel channelMapOut[32]
        ma_dither_mode ditherMode
        ma_channel_mix_mode channelMixMode
        float channelWeights[32][32]
        ma_data_converter_config__resampling resampling

    ma_data_converter_config ma_data_converter_config_init_default()
    ma_data_converter_config ma_data_converter_config_init(ma_format formatIn, ma_format formatOut,
                                  ma_uint32 channelsIn, ma_uint32 channelsOut,
                                  ma_uint32 sampleRateIn, ma_uint32 sampleRateOut)

    ctypedef struct ma_data_converter:
        ma_data_converter_config config
        ma_channel_converter channelConverter
        ma_resampler resampler
        ma_bool8 hasPreFormatConversion
        ma_bool8 hasPostFormatConversion
        ma_bool8 hasChannelConverter
        ma_bool8 hasResampler
        ma_bool8 isPassthrough

    ma_result ma_data_converter_init(const ma_data_converter_config *pConfig,
                                            ma_data_converter *pConverter)
    void ma_data_converter_uninit(ma_data_converter *pConverter)
    ma_result ma_data_converter_process_pcm_frames(
        ma_data_converter *pConverter, const void *pFramesIn,
        ma_uint64 *pFrameCountIn, void *pFramesOut, ma_uint64 *pFrameCountOut)
    ma_result ma_data_converter_set_rate(ma_data_converter *pConverter,
                                                ma_uint32 sampleRateIn,
                                                ma_uint32 sampleRateOut)
    ma_result ma_data_converter_set_rate_ratio(ma_data_converter *pConverter,
                                                      float ratioInOut)
    ma_uint64 ma_data_converter_get_required_input_frame_count(
        const ma_data_converter *pConverter, ma_uint64 outputFrameCount)
    ma_uint64 ma_data_converter_get_expected_output_frame_count(
        const ma_data_converter *pConverter, ma_uint64 inputFrameCount)
    ma_uint64 ma_data_converter_get_input_latency(const ma_data_converter *pConverter)
    ma_uint64 ma_data_converter_get_output_latency(const ma_data_converter *pConverter)

    void ma_pcm_u8_to_s16(void *pOut, const void *pIn, ma_uint64 count,
                                 ma_dither_mode ditherMode)
    void ma_pcm_u8_to_s24(void *pOut, const void *pIn, ma_uint64 count,
                                 ma_dither_mode ditherMode)
    void ma_pcm_u8_to_s32(void *pOut, const void *pIn, ma_uint64 count,
                                 ma_dither_mode ditherMode)
    void ma_pcm_u8_to_f32(void *pOut, const void *pIn, ma_uint64 count,
                                 ma_dither_mode ditherMode)
    void ma_pcm_s16_to_u8(void *pOut, const void *pIn, ma_uint64 count,
                                 ma_dither_mode ditherMode)
    void ma_pcm_s16_to_s24(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_s16_to_s32(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_s16_to_f32(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_s24_to_u8(void *pOut, const void *pIn, ma_uint64 count,
                                 ma_dither_mode ditherMode)
    void ma_pcm_s24_to_s16(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_s24_to_s32(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_s24_to_f32(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_s32_to_u8(void *pOut, const void *pIn, ma_uint64 count,
                                 ma_dither_mode ditherMode)
    void ma_pcm_s32_to_s16(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_s32_to_s24(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_s32_to_f32(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_f32_to_u8(void *pOut, const void *pIn, ma_uint64 count,
                                 ma_dither_mode ditherMode)
    void ma_pcm_f32_to_s16(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_f32_to_s24(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_f32_to_s32(void *pOut, const void *pIn, ma_uint64 count,
                                  ma_dither_mode ditherMode)
    void ma_pcm_convert(void *pOut, ma_format formatOut, const void *pIn,
                               ma_format formatIn, ma_uint64 sampleCount,
                               ma_dither_mode ditherMode)
    void ma_convert_pcm_frames_format(void *pOut, ma_format formatOut,
                                             const void *pIn, ma_format formatIn,
                                             ma_uint64 frameCount,
                                             ma_uint32 channels,
                                             ma_dither_mode ditherMode)

    void ma_deinterleave_pcm_frames(ma_format format, ma_uint32 channels,
                                           ma_uint64 frameCount,
                                           const void *pInterleavedPCMFrames,
                                           void **ppDeinterleavedPCMFrames)

    void ma_interleave_pcm_frames(ma_format format, ma_uint32 channels,
                                         ma_uint64 frameCount,
                                         const void **ppDeinterleavedPCMFrames,
                                         void *pInterleavedPCMFrames)

    void ma_channel_map_init_blank(ma_uint32 channels,
                                          ma_channel *pChannelMap)

    void ma_get_standard_channel_map(ma_standard_channel_map standardChannelMap,
                                ma_uint32 channels, ma_channel *pChannelMap)

    void ma_channel_map_copy(ma_channel *pOut, const ma_channel *pIn,
                                    ma_uint32 channels)

    void ma_channel_map_copy_or_default(ma_channel *pOut,
                                               const ma_channel *pIn,
                                               ma_uint32 channels)

    ma_bool32 ma_channel_map_valid(ma_uint32 channels,
                                          const ma_channel *pChannelMap)

    ma_bool32 ma_channel_map_equal(ma_uint32 channels,
                                          const ma_channel *pChannelMapA,
                                          const ma_channel *pChannelMapB)

    ma_bool32 ma_channel_map_blank(ma_uint32 channels,
                                          const ma_channel *pChannelMap)

    ma_bool32 ma_channel_map_contains_channel_position(ma_uint32 channels,
                                             const ma_channel *pChannelMap,
                                             ma_channel channelPosition)

    ma_uint64 ma_convert_frames(void *pOut, ma_uint64 frameCountOut,
                                       ma_format formatOut, ma_uint32 channelsOut,
                                       ma_uint32 sampleRateOut, const void *pIn,
                                       ma_uint64 frameCountIn, ma_format formatIn,
                                       ma_uint32 channelsIn,
                                       ma_uint32 sampleRateIn)
    ma_uint64 ma_convert_frames_ex(void *pOut, ma_uint64 frameCountOut,
                                          const void *pIn, ma_uint64 frameCountIn,
                                          const ma_data_converter_config *pConfig)



    ctypedef struct ma_rb:
        void *pBuffer
        ma_uint32 subbufferSizeInBytes
        ma_uint32 subbufferCount
        ma_uint32 subbufferStrideInBytes
        ma_uint32 encodedReadOffset
        ma_uint32 encodedWriteOffset
        ma_bool8 ownsBuffer
        ma_bool8 clearOnWriteAcquire
        ma_allocation_callbacks allocationCallbacks


    ma_result ma_rb_init_ex(size_t subbufferSizeInBytes, size_t subbufferCount,
                  size_t subbufferStrideInBytes, void *pOptionalPreallocatedBuffer,
                  const ma_allocation_callbacks *pAllocationCallbacks, ma_rb *pRB)
    ma_result ma_rb_init(size_t bufferSizeInBytes,
                                void *pOptionalPreallocatedBuffer,
                                const ma_allocation_callbacks *pAllocationCallbacks,
                                ma_rb *pRB)
    void ma_rb_uninit(ma_rb *pRB)
    void ma_rb_reset(ma_rb *pRB)
    ma_result ma_rb_acquire_read(ma_rb *pRB, size_t *pSizeInBytes,
                                        void **ppBufferOut)
    ma_result ma_rb_commit_read(ma_rb *pRB, size_t sizeInBytes,
                                       void *pBufferOut)
    ma_result ma_rb_acquire_write(ma_rb *pRB, size_t *pSizeInBytes,
                                         void **ppBufferOut)
    ma_result ma_rb_commit_write(ma_rb *pRB, size_t sizeInBytes,
                                        void *pBufferOut)
    ma_result ma_rb_seek_read(ma_rb *pRB, size_t offsetInBytes)
    ma_result ma_rb_seek_write(ma_rb *pRB, size_t offsetInBytes)
    ma_int32 ma_rb_pointer_distance(ma_rb *pRB)
    ma_uint32 ma_rb_available_read(ma_rb *pRB)
    ma_uint32 ma_rb_available_write(ma_rb *pRB)
    size_t ma_rb_get_subbuffer_size(ma_rb *pRB)
    size_t ma_rb_get_subbuffer_stride(ma_rb *pRB)
    size_t ma_rb_get_subbuffer_offset(ma_rb *pRB, size_t subbufferIndex)
    void *ma_rb_get_subbuffer_ptr(ma_rb *pRB, size_t subbufferIndex,
                                         void *pBuffer)


    ctypedef struct ma_pcm_rb:
        ma_rb rb
        ma_format format
        ma_uint32 channels

    ma_result ma_pcm_rb_init_ex(
        ma_format format, ma_uint32 channels, ma_uint32 subbufferSizeInFrames,
        ma_uint32 subbufferCount, ma_uint32 subbufferStrideInFrames,
        void *pOptionalPreallocatedBuffer,
        const ma_allocation_callbacks *pAllocationCallbacks, ma_pcm_rb *pRB)
    
    ma_result ma_pcm_rb_init(ma_format format, ma_uint32 channels,
                   ma_uint32 bufferSizeInFrames, void *pOptionalPreallocatedBuffer,
                   const ma_allocation_callbacks *pAllocationCallbacks,
                   ma_pcm_rb *pRB)

    void ma_pcm_rb_uninit(ma_pcm_rb *pRB)
    void ma_pcm_rb_reset(ma_pcm_rb *pRB)
    ma_result ma_pcm_rb_acquire_read(ma_pcm_rb *pRB,
                                            ma_uint32 *pSizeInFrames,
                                            void **ppBufferOut)
    ma_result ma_pcm_rb_commit_read(ma_pcm_rb *pRB, ma_uint32 sizeInFrames,
                                           void *pBufferOut)
    ma_result ma_pcm_rb_acquire_write(ma_pcm_rb *pRB,
                                             ma_uint32 *pSizeInFrames,
                                             void **ppBufferOut)
    ma_result ma_pcm_rb_commit_write(ma_pcm_rb *pRB, ma_uint32 sizeInFrames,
                                            void *pBufferOut)
    ma_result ma_pcm_rb_seek_read(ma_pcm_rb *pRB, ma_uint32 offsetInFrames)
    ma_result ma_pcm_rb_seek_write(ma_pcm_rb *pRB, ma_uint32 offsetInFrames)
    ma_int32 ma_pcm_rb_pointer_distance(ma_pcm_rb *pRB)
    ma_uint32 ma_pcm_rb_available_read(ma_pcm_rb *pRB)
    ma_uint32 ma_pcm_rb_available_write(ma_pcm_rb *pRB)
    ma_uint32 ma_pcm_rb_get_subbuffer_size(ma_pcm_rb *pRB)
    ma_uint32 ma_pcm_rb_get_subbuffer_stride(ma_pcm_rb *pRB)
    ma_uint32 ma_pcm_rb_get_subbuffer_offset(ma_pcm_rb *pRB,
                                                    ma_uint32 subbufferIndex)
    void *ma_pcm_rb_get_subbuffer_ptr(ma_pcm_rb *pRB,
                                             ma_uint32 subbufferIndex,
                                             void *pBuffer)

    ctypedef struct ma_duplex_rb:
        ma_pcm_rb rb



    ma_result ma_duplex_rb_init(ma_format captureFormat, ma_uint32 captureChannels,
                      ma_uint32 sampleRate, ma_uint32 captureInternalSampleRate,
                      ma_uint32 captureInternalPeriodSizeInFrames,
                      const ma_allocation_callbacks *pAllocationCallbacks,
                      ma_duplex_rb *pRB)
    ma_result ma_duplex_rb_uninit(ma_duplex_rb *pRB)

    char *ma_result_description(ma_result result)

    void *ma_malloc(size_t sz, const ma_allocation_callbacks *pAllocationCallbacks)

    void *ma_realloc(void *p, size_t sz,
                            const ma_allocation_callbacks *pAllocationCallbacks)

    void ma_free(void *p,
                        const ma_allocation_callbacks *pAllocationCallbacks)

    void * ma_aligned_malloc(size_t sz, size_t alignment,
                      const ma_allocation_callbacks *pAllocationCallbacks)

    void ma_aligned_free(void *p, const ma_allocation_callbacks *pAllocationCallbacks)

    char *ma_get_format_name(ma_format format)

    void ma_blend_f32(float *pOut, float *pInA, float *pInB, float factor,
                             ma_uint32 channels)

    ma_uint32 ma_get_bytes_per_sample(ma_format format)
# 
    inline ma_uint32 ma_get_bytes_per_frame(ma_format format, ma_uint32 channels)

    char *ma_log_level_to_string(ma_uint32 logLevel)


    ctypedef enum ma_backend:
        ma_backend_wasapi
        ma_backend_dsound
        ma_backend_winmm
        ma_backend_coreaudio
        ma_backend_sndio
        ma_backend_audio4
        ma_backend_oss
        ma_backend_pulseaudio
        ma_backend_alsa
        ma_backend_jack
        ma_backend_aaudio
        ma_backend_opensl
        ma_backend_webaudio
        ma_backend_custom
        ma_backend_null


    ctypedef void (*ma_device_callback_proc)(ma_device *pDevice, void *pOutput,
                                            const void *pInput,
                                            ma_uint32 frameCount)

    ctypedef void (*ma_stop_proc)(ma_device *pDevice)

    ctypedef void (*ma_log_proc)(ma_context *pContext, ma_device *pDevice,
                                ma_uint32 logLevel, const char *message)

    ctypedef enum ma_device_type:
        ma_device_type_playback = 1
        ma_device_type_capture  = 2
        ma_device_type_duplex   = 3
        ma_device_type_loopback = 4

    ctypedef enum ma_share_mode:
        ma_share_mode_shared = 0
        ma_share_mode_exclusive

    ctypedef enum ma_ios_session_category:
        ma_ios_session_category_default = 0
        ma_ios_session_category_none
        ma_ios_session_category_ambient
        ma_ios_session_category_solo_ambient
        ma_ios_session_category_playback
        ma_ios_session_category_record
        ma_ios_session_category_play_and_record
        ma_ios_session_category_multi_route

    ctypedef enum ma_ios_session_category_option:
        ma_ios_session_category_option_mix_with_others = 0x01
        ma_ios_session_category_option_duck_others = 0x02
        ma_ios_session_category_option_allow_bluetooth = 0x04
        ma_ios_session_category_option_default_to_speaker = 0x08
        ma_ios_session_category_option_interrupt_spoken_audio_and_mix_with_others = 0x11
        ma_ios_session_category_option_allow_bluetooth_a2dp = 0x20
        ma_ios_session_category_option_allow_air_play = 0x40



    ctypedef enum ma_opensl_stream_type:
        ma_opensl_stream_type_default = 0
        ma_opensl_stream_type_voice
        ma_opensl_stream_type_system
        ma_opensl_stream_type_ring
        ma_opensl_stream_type_media
        ma_opensl_stream_type_alarm
        ma_opensl_stream_type_notification

    ctypedef enum ma_opensl_recording_preset:
        ma_opensl_recording_preset_default = 0
        ma_opensl_recording_preset_generic
        ma_opensl_recording_preset_camcorder
        ma_opensl_recording_preset_voice_recognition
        ma_opensl_recording_preset_voice_communication
        ma_opensl_recording_preset_voice_unprocessed

    ctypedef enum ma_aaudio_usage:
        ma_aaudio_usage_default = 0
        ma_aaudio_usage_announcement
        ma_aaudio_usage_emergency
        ma_aaudio_usage_safety
        ma_aaudio_usage_vehicle_status
        ma_aaudio_usage_alarm
        ma_aaudio_usage_assistance_accessibility
        ma_aaudio_usage_assistance_navigation_guidance
        ma_aaudio_usage_assistance_sonification
        ma_aaudio_usage_assitant
        ma_aaudio_usage_game
        ma_aaudio_usage_media
        ma_aaudio_usage_notification
        ma_aaudio_usage_notification_event
        ma_aaudio_usage_notification_ringtone
        ma_aaudio_usage_voice_communication
        ma_aaudio_usage_voice_communication_signalling


    ctypedef enum ma_aaudio_content_type:
        ma_aaudio_content_type_default = 0
        ma_aaudio_content_type_movie
        ma_aaudio_content_type_music
        ma_aaudio_content_type_sonification
        ma_aaudio_content_type_speech


    ctypedef enum ma_aaudio_input_preset:
        ma_aaudio_input_preset_default = 0
        ma_aaudio_input_preset_generic
        ma_aaudio_input_preset_camcorder
        ma_aaudio_input_preset_unprocessed
        ma_aaudio_input_preset_voice_recognition
        ma_aaudio_input_preset_voice_communication
        ma_aaudio_input_preset_voice_performance


    ctypedef union ma_timer:
        ma_int64 counter
        double counterD


    cdef union ma_device_id__custom:
        int i
        char s[256]
        void *p

    ctypedef union ma_device_id:
        # wchar_t wasapi[64]
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
        ma_device_id__custom custom
        int nullbackend

    ctypedef struct ma_context_config



    cdef struct ma_device_info__nativeDataFormats:
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        ma_uint32 flags

    ctypedef struct ma_device_info:
        ma_device_id id
        char name[256]
        ma_bool32 isDefault

        ma_uint32 formatCount
        ma_format formats[6]
        ma_uint32 minChannels
        ma_uint32 maxChannels
        ma_uint32 minSampleRate
        ma_uint32 maxSampleRate

        ma_uint32 nativeDataFormatCount
        ma_device_info__nativeDataFormats nativeDataFormats[64]
    

    # ----------------------------------------------------------------------
    # RESTART HERE <--
    # ----------------------------------------------------------------------

    cdef struct ma_device_config__resampling__linear:
        ma_uint32 lpfOrder

    cdef struct ma_device_config__resampling__speex:
        int quality

    cdef struct ma_device_config__resampling:
        # ma_resample_algorithm algorithm
        ma_device_config__resampling__linear linear
        ma_device_config__resampling__speex speex

    cdef struct ma_device_config__playback:
        ma_device_id* pDeviceID
        ma_format format
        ma_uint32 channels
        ma_channel channelMap[MA_MAX_CHANNELS]
        ma_channel_mix_mode channelMixMode
        ma_share_mode shareMode

    cdef struct ma_device_config__capture:
        ma_device_id* pDeviceID
        ma_format format
        ma_uint32 channels
        ma_channel channelMap[MA_MAX_CHANNELS]
        ma_channel_mix_mode channelMixMode
        ma_share_mode shareMode

    cdef struct ma_device_config__wasapi:
        ma_bool8 noAutoConvertSRC
        ma_bool8 noDefaultQualitySRC
        ma_bool8 noAutoStreamRouting
        ma_bool8 noHardwareOffloading

    cdef struct ma_device_config__alsa:
        ma_bool32 noMMap
        ma_bool32 noAutoFormat
        ma_bool32 noAutoChannels
        ma_bool32 noAutoResample

    cdef struct ma_device_config__pulse:
        char *pStreamNamePlayback
        char *pStreamNameCapture

    cdef struct ma_device_config__coreaudio:
        ma_bool32 allowNominalSampleRateChange

    cdef struct ma_device_config__opensl:
        ma_opensl_stream_type streamType
        ma_opensl_recording_preset recordingPreset

    cdef struct ma_device_config__aaudio:
            ma_aaudio_usage usage
            ma_aaudio_content_type contentType
            ma_aaudio_input_preset inputPreset

    ctypedef struct ma_device_config:
        ma_device_type deviceType
        ma_uint32 sampleRate
        ma_uint32 periodSizeInFrames
        ma_uint32 periodSizeInMilliseconds
        ma_uint32 periods
        ma_performance_profile performanceProfile
        ma_bool8 noPreZeroedOutputBuffer
        ma_bool8 noClip
        ma_device_callback_proc dataCallback
        ma_stop_proc stopCallback
        void *pUserData
        ma_device_config__resampling resampling
        ma_device_config__playback playback
        ma_device_config__capture capture
        ma_device_config__wasapi wasapi
        ma_device_config__alsa alsa
        ma_device_config__pulse pulse
        ma_device_config__coreaudio coreaudio
        ma_device_config__opensl opensl
        ma_device_config__aaudio aaudio

    ctypedef ma_bool32 (*ma_enum_devices_callback_proc)(ma_context *pContext,
                                                       ma_device_type deviceType,
                                                       const ma_device_info *pInfo,
                                                       void *pUserData)

    ctypedef struct ma_device_descriptor:
        const ma_device_id *pDeviceID
        ma_share_mode shareMode
        ma_format format
        ma_uint32 channels
        ma_uint32 sampleRate
        ma_channel channelMap[32]
        ma_uint32 periodSizeInFrames
        ma_uint32 periodSizeInMilliseconds
        ma_uint32 periodCount

    ctypedef struct ma_backend_callbacks:
        ma_result (*onContextInit)(ma_context *pContext,
                                   const ma_context_config *pConfig,
                                   ma_backend_callbacks *pCallbacks)
        ma_result (*onContextUninit)(ma_context *pContext)
        ma_result (*onContextEnumerateDevices)(
            ma_context *pContext, ma_enum_devices_callback_proc callback,
            void *pUserData)
        ma_result (*onContextGetDeviceInfo)(ma_context *pContext,
                                            ma_device_type deviceType,
                                            const ma_device_id *pDeviceID,
                                            ma_device_info *pDeviceInfo)
        ma_result (*onDeviceInit)(ma_device *pDevice,
                                  const ma_device_config *pConfig,
                                  ma_device_descriptor *pDescriptorPlayback,
                                  ma_device_descriptor *pDescriptorCapture)
        ma_result (*onDeviceUninit)(ma_device *pDevice)
        ma_result (*onDeviceStart)(ma_device *pDevice)
        ma_result (*onDeviceStop)(ma_device *pDevice)
        ma_result (*onDeviceRead)(ma_device *pDevice, void *pFrames,
                                  ma_uint32 frameCount, ma_uint32 *pFramesRead)
        ma_result (*onDeviceWrite)(ma_device *pDevice, const void *pFrames,
                                   ma_uint32 frameCount, ma_uint32 *pFramesWritten)
        ma_result (*onDeviceDataLoop)(ma_device *pDevice)
        ma_result (*onDeviceDataLoopWakeup)(ma_device *pDevice)







