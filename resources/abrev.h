

typedef signed char ma_int8;
typedef unsigned char ma_uint8;
typedef signed short ma_int16;
typedef unsigned short ma_uint16;
typedef signed int ma_int32;
typedef unsigned int ma_uint32;






typedef signed long long ma_int64;
typedef unsigned long long ma_uint64;



typedef ma_uint64 ma_uintptr;

typedef ma_uint8 ma_bool8;
typedef ma_uint32 ma_bool32;

typedef void *ma_handle;
typedef void *ma_ptr;
typedef void (*ma_proc)(void);

typedef struct ma_context ma_context;
typedef struct ma_device ma_device;

typedef ma_uint8 ma_channel;

typedef int ma_result;

typedef enum { ma_stream_format_pcm = 0 } ma_stream_format;

typedef enum {
    ma_stream_layout_interleaved = 0,
    ma_stream_layout_deinterleaved
} ma_stream_layout;

typedef enum {
    ma_dither_mode_none = 0,
    ma_dither_mode_rectangle,
    ma_dither_mode_triangle
} ma_dither_mode;

typedef enum {

    ma_format_unknown = 0,
    ma_format_u8 = 1,
    ma_format_s16 = 2,
    ma_format_s24 = 3,
    ma_format_s32 = 4,
    ma_format_f32 = 5,
    ma_format_count
} ma_format;

typedef enum {

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
    ma_standard_sample_rate_11025 = 11250,
    ma_standard_sample_rate_8000 = 8000,

    ma_standard_sample_rate_352800 = 352800,
    ma_standard_sample_rate_384000 = 384000,

    ma_standard_sample_rate_min = ma_standard_sample_rate_8000,
    ma_standard_sample_rate_max = ma_standard_sample_rate_384000,
    ma_standard_sample_rate_count = 14
} ma_standard_sample_rate;

typedef enum {
    ma_channel_mix_mode_rectangular = 0,
    ma_channel_mix_mode_simple,
    ma_channel_mix_mode_custom_weights,
    ma_channel_mix_mode_planar_blend = ma_channel_mix_mode_rectangular,
    ma_channel_mix_mode_default = ma_channel_mix_mode_planar_blend
} ma_channel_mix_mode;

typedef enum {
    ma_standard_channel_map_microsoft,
    ma_standard_channel_map_alsa,
    ma_standard_channel_map_rfc3551,
    ma_standard_channel_map_flac,
    ma_standard_channel_map_vorbis,
    ma_standard_channel_map_sound4,
    ma_standard_channel_map_sndio,
    ma_standard_channel_map_webaudio = ma_standard_channel_map_flac,
    ma_standard_channel_map_default = ma_standard_channel_map_microsoft
} ma_standard_channel_map;

typedef enum {
    ma_performance_profile_low_latency = 0,
    ma_performance_profile_conservative
} ma_performance_profile;

typedef struct {
    void *pUserData;
    void *(*onMalloc)(size_t sz, void *pUserData);
    void *(*onRealloc)(void *p, size_t sz, void *pUserData);
    void (*onFree)(void *p, void *pUserData);
} ma_allocation_callbacks;

typedef struct {
    ma_int32 state;
} ma_lcg;

typedef enum {
    ma_thread_priority_idle = -5,
    ma_thread_priority_lowest = -4,
    ma_thread_priority_low = -3,
    ma_thread_priority_normal = -2,
    ma_thread_priority_high = -1,
    ma_thread_priority_highest = 0,
    ma_thread_priority_realtime = 1,
    ma_thread_priority_default = 0
} ma_thread_priority;

typedef ma_uint32 ma_spinlock;

typedef pthread_t ma_thread;

typedef pthread_mutex_t ma_mutex;

typedef struct {
    ma_uint32 value;
    pthread_mutex_t lock;
    pthread_cond_t cond;
} ma_event;

typedef struct {
    int value;
    pthread_mutex_t lock;
    pthread_cond_t cond;
} ma_semaphore;

extern void ma_version(ma_uint32 *pMajor, ma_uint32 *pMinor,
                       ma_uint32 *pRevision);

extern const char *ma_version_string(void);

typedef union {
    float f32;
    ma_int32 s32;
} ma_biquad_coefficient;

typedef struct {
    ma_format format;
    ma_uint32 channels;
    double b0;
    double b1;
    double b2;
    double a0;
    double a1;
    double a2;
} ma_biquad_config;

extern ma_biquad_config ma_biquad_config_init(ma_format format,
                                              ma_uint32 channels, double b0,
                                              double b1, double b2, double a0,
                                              double a1, double a2);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_biquad_coefficient b0;
    ma_biquad_coefficient b1;
    ma_biquad_coefficient b2;
    ma_biquad_coefficient a1;
    ma_biquad_coefficient a2;
    ma_biquad_coefficient r1[32];
    ma_biquad_coefficient r2[32];
} ma_biquad;

extern ma_result ma_biquad_init(const ma_biquad_config *pConfig,
                                ma_biquad *pBQ);
extern ma_result ma_biquad_reinit(const ma_biquad_config *pConfig,
                                  ma_biquad *pBQ);
extern ma_result ma_biquad_process_pcm_frames(ma_biquad *pBQ, void *pFramesOut,
                                              const void *pFramesIn,
                                              ma_uint64 frameCount);
extern ma_uint32 ma_biquad_get_latency(const ma_biquad *pBQ);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double cutoffFrequency;
    double q;
} ma_lpf1_config, ma_lpf2_config;

extern ma_lpf1_config ma_lpf1_config_init(ma_format format, ma_uint32 channels,
                                          ma_uint32 sampleRate,
                                          double cutoffFrequency);
extern ma_lpf2_config ma_lpf2_config_init(ma_format format, ma_uint32 channels,
                                          ma_uint32 sampleRate,
                                          double cutoffFrequency, double q);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_biquad_coefficient a;
    ma_biquad_coefficient r1[32];
} ma_lpf1;

extern ma_result ma_lpf1_init(const ma_lpf1_config *pConfig, ma_lpf1 *pLPF);
extern ma_result ma_lpf1_reinit(const ma_lpf1_config *pConfig, ma_lpf1 *pLPF);
extern ma_result ma_lpf1_process_pcm_frames(ma_lpf1 *pLPF, void *pFramesOut,
                                            const void *pFramesIn,
                                            ma_uint64 frameCount);
extern ma_uint32 ma_lpf1_get_latency(const ma_lpf1 *pLPF);

typedef struct {
    ma_biquad bq;
} ma_lpf2;

extern ma_result ma_lpf2_init(const ma_lpf2_config *pConfig, ma_lpf2 *pLPF);
extern ma_result ma_lpf2_reinit(const ma_lpf2_config *pConfig, ma_lpf2 *pLPF);
extern ma_result ma_lpf2_process_pcm_frames(ma_lpf2 *pLPF, void *pFramesOut,
                                            const void *pFramesIn,
                                            ma_uint64 frameCount);
extern ma_uint32 ma_lpf2_get_latency(const ma_lpf2 *pLPF);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double cutoffFrequency;
    ma_uint32 order;
} ma_lpf_config;

extern ma_lpf_config ma_lpf_config_init(ma_format format, ma_uint32 channels,
                                        ma_uint32 sampleRate,
                                        double cutoffFrequency,
                                        ma_uint32 order);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    ma_uint32 lpf1Count;
    ma_uint32 lpf2Count;
    ma_lpf1 lpf1[1];
    ma_lpf2 lpf2[8 / 2];
} ma_lpf;

extern ma_result ma_lpf_init(const ma_lpf_config *pConfig, ma_lpf *pLPF);
extern ma_result ma_lpf_reinit(const ma_lpf_config *pConfig, ma_lpf *pLPF);
extern ma_result ma_lpf_process_pcm_frames(ma_lpf *pLPF, void *pFramesOut,
                                           const void *pFramesIn,
                                           ma_uint64 frameCount);
extern ma_uint32 ma_lpf_get_latency(const ma_lpf *pLPF);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double cutoffFrequency;
    double q;
} ma_hpf1_config, ma_hpf2_config;

extern ma_hpf1_config ma_hpf1_config_init(ma_format format, ma_uint32 channels,
                                          ma_uint32 sampleRate,
                                          double cutoffFrequency);
extern ma_hpf2_config ma_hpf2_config_init(ma_format format, ma_uint32 channels,
                                          ma_uint32 sampleRate,
                                          double cutoffFrequency, double q);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_biquad_coefficient a;
    ma_biquad_coefficient r1[32];
} ma_hpf1;

extern ma_result ma_hpf1_init(const ma_hpf1_config *pConfig, ma_hpf1 *pHPF);
extern ma_result ma_hpf1_reinit(const ma_hpf1_config *pConfig, ma_hpf1 *pHPF);
extern ma_result ma_hpf1_process_pcm_frames(ma_hpf1 *pHPF, void *pFramesOut,
                                            const void *pFramesIn,
                                            ma_uint64 frameCount);
extern ma_uint32 ma_hpf1_get_latency(const ma_hpf1 *pHPF);

typedef struct {
    ma_biquad bq;
} ma_hpf2;

extern ma_result ma_hpf2_init(const ma_hpf2_config *pConfig, ma_hpf2 *pHPF);
extern ma_result ma_hpf2_reinit(const ma_hpf2_config *pConfig, ma_hpf2 *pHPF);
extern ma_result ma_hpf2_process_pcm_frames(ma_hpf2 *pHPF, void *pFramesOut,
                                            const void *pFramesIn,
                                            ma_uint64 frameCount);
extern ma_uint32 ma_hpf2_get_latency(const ma_hpf2 *pHPF);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double cutoffFrequency;
    ma_uint32 order;
} ma_hpf_config;

extern ma_hpf_config ma_hpf_config_init(ma_format format, ma_uint32 channels,
                                        ma_uint32 sampleRate,
                                        double cutoffFrequency,
                                        ma_uint32 order);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    ma_uint32 hpf1Count;
    ma_uint32 hpf2Count;
    ma_hpf1 hpf1[1];
    ma_hpf2 hpf2[8 / 2];
} ma_hpf;

extern ma_result ma_hpf_init(const ma_hpf_config *pConfig, ma_hpf *pHPF);
extern ma_result ma_hpf_reinit(const ma_hpf_config *pConfig, ma_hpf *pHPF);
extern ma_result ma_hpf_process_pcm_frames(ma_hpf *pHPF, void *pFramesOut,
                                           const void *pFramesIn,
                                           ma_uint64 frameCount);
extern ma_uint32 ma_hpf_get_latency(const ma_hpf *pHPF);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double cutoffFrequency;
    double q;
} ma_bpf2_config;

extern ma_bpf2_config ma_bpf2_config_init(ma_format format, ma_uint32 channels,
                                          ma_uint32 sampleRate,
                                          double cutoffFrequency, double q);

typedef struct {
    ma_biquad bq;
} ma_bpf2;

extern ma_result ma_bpf2_init(const ma_bpf2_config *pConfig, ma_bpf2 *pBPF);
extern ma_result ma_bpf2_reinit(const ma_bpf2_config *pConfig, ma_bpf2 *pBPF);
extern ma_result ma_bpf2_process_pcm_frames(ma_bpf2 *pBPF, void *pFramesOut,
                                            const void *pFramesIn,
                                            ma_uint64 frameCount);
extern ma_uint32 ma_bpf2_get_latency(const ma_bpf2 *pBPF);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double cutoffFrequency;
    ma_uint32 order;
} ma_bpf_config;

extern ma_bpf_config ma_bpf_config_init(ma_format format, ma_uint32 channels,
                                        ma_uint32 sampleRate,
                                        double cutoffFrequency,
                                        ma_uint32 order);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 bpf2Count;
    ma_bpf2 bpf2[8 / 2];
} ma_bpf;

extern ma_result ma_bpf_init(const ma_bpf_config *pConfig, ma_bpf *pBPF);
extern ma_result ma_bpf_reinit(const ma_bpf_config *pConfig, ma_bpf *pBPF);
extern ma_result ma_bpf_process_pcm_frames(ma_bpf *pBPF, void *pFramesOut,
                                           const void *pFramesIn,
                                           ma_uint64 frameCount);
extern ma_uint32 ma_bpf_get_latency(const ma_bpf *pBPF);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double q;
    double frequency;
} ma_notch2_config;

extern ma_notch2_config ma_notch2_config_init(ma_format format,
                                              ma_uint32 channels,
                                              ma_uint32 sampleRate, double q,
                                              double frequency);

typedef struct {
    ma_biquad bq;
} ma_notch2;

extern ma_result ma_notch2_init(const ma_notch2_config *pConfig,
                                ma_notch2 *pFilter);
extern ma_result ma_notch2_reinit(const ma_notch2_config *pConfig,
                                  ma_notch2 *pFilter);
extern ma_result ma_notch2_process_pcm_frames(ma_notch2 *pFilter,
                                              void *pFramesOut,
                                              const void *pFramesIn,
                                              ma_uint64 frameCount);
extern ma_uint32 ma_notch2_get_latency(const ma_notch2 *pFilter);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double gainDB;
    double q;
    double frequency;
} ma_peak2_config;

extern ma_peak2_config ma_peak2_config_init(ma_format format,
                                            ma_uint32 channels,
                                            ma_uint32 sampleRate, double gainDB,
                                            double q, double frequency);

typedef struct {
    ma_biquad bq;
} ma_peak2;

extern ma_result ma_peak2_init(const ma_peak2_config *pConfig,
                               ma_peak2 *pFilter);
extern ma_result ma_peak2_reinit(const ma_peak2_config *pConfig,
                                 ma_peak2 *pFilter);
extern ma_result ma_peak2_process_pcm_frames(ma_peak2 *pFilter,
                                             void *pFramesOut,
                                             const void *pFramesIn,
                                             ma_uint64 frameCount);
extern ma_uint32 ma_peak2_get_latency(const ma_peak2 *pFilter);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double gainDB;
    double shelfSlope;
    double frequency;
} ma_loshelf2_config;

extern ma_loshelf2_config
ma_loshelf2_config_init(ma_format format, ma_uint32 channels,
                        ma_uint32 sampleRate, double gainDB, double shelfSlope,
                        double frequency);

typedef struct {
    ma_biquad bq;
} ma_loshelf2;

extern ma_result ma_loshelf2_init(const ma_loshelf2_config *pConfig,
                                  ma_loshelf2 *pFilter);
extern ma_result ma_loshelf2_reinit(const ma_loshelf2_config *pConfig,
                                    ma_loshelf2 *pFilter);
extern ma_result ma_loshelf2_process_pcm_frames(ma_loshelf2 *pFilter,
                                                void *pFramesOut,
                                                const void *pFramesIn,
                                                ma_uint64 frameCount);
extern ma_uint32 ma_loshelf2_get_latency(const ma_loshelf2 *pFilter);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    double gainDB;
    double shelfSlope;
    double frequency;
} ma_hishelf2_config;

extern ma_hishelf2_config
ma_hishelf2_config_init(ma_format format, ma_uint32 channels,
                        ma_uint32 sampleRate, double gainDB, double shelfSlope,
                        double frequency);

typedef struct {
    ma_biquad bq;
} ma_hishelf2;

extern ma_result ma_hishelf2_init(const ma_hishelf2_config *pConfig,
                                  ma_hishelf2 *pFilter);
extern ma_result ma_hishelf2_reinit(const ma_hishelf2_config *pConfig,
                                    ma_hishelf2 *pFilter);
extern ma_result ma_hishelf2_process_pcm_frames(ma_hishelf2 *pFilter,
                                                void *pFramesOut,
                                                const void *pFramesIn,
                                                ma_uint64 frameCount);
extern ma_uint32 ma_hishelf2_get_latency(const ma_hishelf2 *pFilter);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRateIn;
    ma_uint32 sampleRateOut;
    ma_uint32 lpfOrder;
    double lpfNyquistFactor;
} ma_linear_resampler_config;

extern ma_linear_resampler_config
ma_linear_resampler_config_init(ma_format format, ma_uint32 channels,
                                ma_uint32 sampleRateIn,
                                ma_uint32 sampleRateOut);

typedef struct {
    ma_linear_resampler_config config;
    ma_uint32 inAdvanceInt;
    ma_uint32 inAdvanceFrac;
    ma_uint32 inTimeInt;
    ma_uint32 inTimeFrac;
    union {
        float f32[32];
        ma_int16 s16[32];
    } x0;
    union {
        float f32[32];
        ma_int16 s16[32];
    } x1;
    ma_lpf lpf;
} ma_linear_resampler;

extern ma_result
ma_linear_resampler_init(const ma_linear_resampler_config *pConfig,
                         ma_linear_resampler *pResampler);
extern void ma_linear_resampler_uninit(ma_linear_resampler *pResampler);
extern ma_result ma_linear_resampler_process_pcm_frames(
    ma_linear_resampler *pResampler, const void *pFramesIn,
    ma_uint64 *pFrameCountIn, void *pFramesOut, ma_uint64 *pFrameCountOut);
extern ma_result ma_linear_resampler_set_rate(ma_linear_resampler *pResampler,
                                              ma_uint32 sampleRateIn,
                                              ma_uint32 sampleRateOut);
extern ma_result
ma_linear_resampler_set_rate_ratio(ma_linear_resampler *pResampler,
                                   float ratioInOut);
extern ma_uint64 ma_linear_resampler_get_required_input_frame_count(
    const ma_linear_resampler *pResampler, ma_uint64 outputFrameCount);
extern ma_uint64 ma_linear_resampler_get_expected_output_frame_count(
    const ma_linear_resampler *pResampler, ma_uint64 inputFrameCount);
extern ma_uint64
ma_linear_resampler_get_input_latency(const ma_linear_resampler *pResampler);
extern ma_uint64
ma_linear_resampler_get_output_latency(const ma_linear_resampler *pResampler);

typedef enum {
    ma_resample_algorithm_linear = 0,
    ma_resample_algorithm_speex
} ma_resample_algorithm;

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRateIn;
    ma_uint32 sampleRateOut;
    ma_resample_algorithm algorithm;
    struct {
        ma_uint32 lpfOrder;
        double lpfNyquistFactor;
    } linear;
    struct {
        int quality;
    } speex;
} ma_resampler_config;

extern ma_resampler_config
ma_resampler_config_init(ma_format format, ma_uint32 channels,
                         ma_uint32 sampleRateIn, ma_uint32 sampleRateOut,
                         ma_resample_algorithm algorithm);

typedef struct {
    ma_resampler_config config;
    union {
        ma_linear_resampler linear;
        struct {
            void *pSpeexResamplerState;
        } speex;
    } state;
} ma_resampler;

extern ma_result ma_resampler_init(const ma_resampler_config *pConfig,
                                   ma_resampler *pResampler);

extern void ma_resampler_uninit(ma_resampler *pResampler);

extern ma_result ma_resampler_process_pcm_frames(ma_resampler *pResampler,
                                                 const void *pFramesIn,
                                                 ma_uint64 *pFrameCountIn,
                                                 void *pFramesOut,
                                                 ma_uint64 *pFrameCountOut);

extern ma_result ma_resampler_set_rate(ma_resampler *pResampler,
                                       ma_uint32 sampleRateIn,
                                       ma_uint32 sampleRateOut);

extern ma_result ma_resampler_set_rate_ratio(ma_resampler *pResampler,
                                             float ratio);

extern ma_uint64
ma_resampler_get_required_input_frame_count(const ma_resampler *pResampler,
                                            ma_uint64 outputFrameCount);

extern ma_uint64
ma_resampler_get_expected_output_frame_count(const ma_resampler *pResampler,
                                             ma_uint64 inputFrameCount);

extern ma_uint64 ma_resampler_get_input_latency(const ma_resampler *pResampler);

extern ma_uint64
ma_resampler_get_output_latency(const ma_resampler *pResampler);

typedef struct {
    ma_format format;
    ma_uint32 channelsIn;
    ma_uint32 channelsOut;
    ma_channel channelMapIn[32];
    ma_channel channelMapOut[32];
    ma_channel_mix_mode mixingMode;
    float weights[32][32];
} ma_channel_converter_config;

extern ma_channel_converter_config ma_channel_converter_config_init(
    ma_format format, ma_uint32 channelsIn, const ma_channel *pChannelMapIn,
    ma_uint32 channelsOut, const ma_channel *pChannelMapOut,
    ma_channel_mix_mode mixingMode);

typedef struct {
    ma_format format;
    ma_uint32 channelsIn;
    ma_uint32 channelsOut;
    ma_channel channelMapIn[32];
    ma_channel channelMapOut[32];
    ma_channel_mix_mode mixingMode;
    union {
        float f32[32][32];
        ma_int32 s16[32][32];
    } weights;
    ma_bool8 isPassthrough;
    ma_bool8 isSimpleShuffle;
    ma_bool8 isSimpleMonoExpansion;
    ma_bool8 isStereoToMono;
    ma_uint8 shuffleTable[32];
} ma_channel_converter;

extern ma_result
ma_channel_converter_init(const ma_channel_converter_config *pConfig,
                          ma_channel_converter *pConverter);
extern void ma_channel_converter_uninit(ma_channel_converter *pConverter);
extern ma_result
ma_channel_converter_process_pcm_frames(ma_channel_converter *pConverter,
                                        void *pFramesOut, const void *pFramesIn,
                                        ma_uint64 frameCount);

typedef struct {
    ma_format formatIn;
    ma_format formatOut;
    ma_uint32 channelsIn;
    ma_uint32 channelsOut;
    ma_uint32 sampleRateIn;
    ma_uint32 sampleRateOut;
    ma_channel channelMapIn[32];
    ma_channel channelMapOut[32];
    ma_dither_mode ditherMode;
    ma_channel_mix_mode channelMixMode;
    float channelWeights[32][32];
    struct {
        ma_resample_algorithm algorithm;
        ma_bool32 allowDynamicSampleRate;
        struct {
            ma_uint32 lpfOrder;
            double lpfNyquistFactor;
        } linear;
        struct {
            int quality;
        } speex;
    } resampling;
} ma_data_converter_config;

extern ma_data_converter_config ma_data_converter_config_init_default(void);
extern ma_data_converter_config
ma_data_converter_config_init(ma_format formatIn, ma_format formatOut,
                              ma_uint32 channelsIn, ma_uint32 channelsOut,
                              ma_uint32 sampleRateIn, ma_uint32 sampleRateOut);

typedef struct {
    ma_data_converter_config config;
    ma_channel_converter channelConverter;
    ma_resampler resampler;
    ma_bool8 hasPreFormatConversion;
    ma_bool8 hasPostFormatConversion;
    ma_bool8 hasChannelConverter;
    ma_bool8 hasResampler;
    ma_bool8 isPassthrough;
} ma_data_converter;

extern ma_result ma_data_converter_init(const ma_data_converter_config *pConfig,
                                        ma_data_converter *pConverter);
extern void ma_data_converter_uninit(ma_data_converter *pConverter);
extern ma_result ma_data_converter_process_pcm_frames(
    ma_data_converter *pConverter, const void *pFramesIn,
    ma_uint64 *pFrameCountIn, void *pFramesOut, ma_uint64 *pFrameCountOut);
extern ma_result ma_data_converter_set_rate(ma_data_converter *pConverter,
                                            ma_uint32 sampleRateIn,
                                            ma_uint32 sampleRateOut);
extern ma_result ma_data_converter_set_rate_ratio(ma_data_converter *pConverter,
                                                  float ratioInOut);
extern ma_uint64 ma_data_converter_get_required_input_frame_count(
    const ma_data_converter *pConverter, ma_uint64 outputFrameCount);
extern ma_uint64 ma_data_converter_get_expected_output_frame_count(
    const ma_data_converter *pConverter, ma_uint64 inputFrameCount);
extern ma_uint64
ma_data_converter_get_input_latency(const ma_data_converter *pConverter);
extern ma_uint64
ma_data_converter_get_output_latency(const ma_data_converter *pConverter);

extern void ma_pcm_u8_to_s16(void *pOut, const void *pIn, ma_uint64 count,
                             ma_dither_mode ditherMode);
extern void ma_pcm_u8_to_s24(void *pOut, const void *pIn, ma_uint64 count,
                             ma_dither_mode ditherMode);
extern void ma_pcm_u8_to_s32(void *pOut, const void *pIn, ma_uint64 count,
                             ma_dither_mode ditherMode);
extern void ma_pcm_u8_to_f32(void *pOut, const void *pIn, ma_uint64 count,
                             ma_dither_mode ditherMode);
extern void ma_pcm_s16_to_u8(void *pOut, const void *pIn, ma_uint64 count,
                             ma_dither_mode ditherMode);
extern void ma_pcm_s16_to_s24(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_s16_to_s32(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_s16_to_f32(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_s24_to_u8(void *pOut, const void *pIn, ma_uint64 count,
                             ma_dither_mode ditherMode);
extern void ma_pcm_s24_to_s16(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_s24_to_s32(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_s24_to_f32(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_s32_to_u8(void *pOut, const void *pIn, ma_uint64 count,
                             ma_dither_mode ditherMode);
extern void ma_pcm_s32_to_s16(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_s32_to_s24(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_s32_to_f32(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_f32_to_u8(void *pOut, const void *pIn, ma_uint64 count,
                             ma_dither_mode ditherMode);
extern void ma_pcm_f32_to_s16(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_f32_to_s24(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_f32_to_s32(void *pOut, const void *pIn, ma_uint64 count,
                              ma_dither_mode ditherMode);
extern void ma_pcm_convert(void *pOut, ma_format formatOut, const void *pIn,
                           ma_format formatIn, ma_uint64 sampleCount,
                           ma_dither_mode ditherMode);
extern void ma_convert_pcm_frames_format(void *pOut, ma_format formatOut,
                                         const void *pIn, ma_format formatIn,
                                         ma_uint64 frameCount,
                                         ma_uint32 channels,
                                         ma_dither_mode ditherMode);

extern void ma_deinterleave_pcm_frames(ma_format format, ma_uint32 channels,
                                       ma_uint64 frameCount,
                                       const void *pInterleavedPCMFrames,
                                       void **ppDeinterleavedPCMFrames);

extern void ma_interleave_pcm_frames(ma_format format, ma_uint32 channels,
                                     ma_uint64 frameCount,
                                     const void **ppDeinterleavedPCMFrames,
                                     void *pInterleavedPCMFrames);

extern void ma_channel_map_init_blank(ma_uint32 channels,
                                      ma_channel *pChannelMap);

extern void
ma_get_standard_channel_map(ma_standard_channel_map standardChannelMap,
                            ma_uint32 channels, ma_channel *pChannelMap);

extern void ma_channel_map_copy(ma_channel *pOut, const ma_channel *pIn,
                                ma_uint32 channels);

extern void ma_channel_map_copy_or_default(ma_channel *pOut,
                                           const ma_channel *pIn,
                                           ma_uint32 channels);

extern ma_bool32 ma_channel_map_valid(ma_uint32 channels,
                                      const ma_channel *pChannelMap);

extern ma_bool32 ma_channel_map_equal(ma_uint32 channels,
                                      const ma_channel *pChannelMapA,
                                      const ma_channel *pChannelMapB);

extern ma_bool32 ma_channel_map_blank(ma_uint32 channels,
                                      const ma_channel *pChannelMap);

extern ma_bool32
ma_channel_map_contains_channel_position(ma_uint32 channels,
                                         const ma_channel *pChannelMap,
                                         ma_channel channelPosition);

extern ma_uint64 ma_convert_frames(void *pOut, ma_uint64 frameCountOut,
                                   ma_format formatOut, ma_uint32 channelsOut,
                                   ma_uint32 sampleRateOut, const void *pIn,
                                   ma_uint64 frameCountIn, ma_format formatIn,
                                   ma_uint32 channelsIn,
                                   ma_uint32 sampleRateIn);
extern ma_uint64 ma_convert_frames_ex(void *pOut, ma_uint64 frameCountOut,
                                      const void *pIn, ma_uint64 frameCountIn,
                                      const ma_data_converter_config *pConfig);

typedef struct {
    void *pBuffer;
    ma_uint32 subbufferSizeInBytes;
    ma_uint32 subbufferCount;
    ma_uint32 subbufferStrideInBytes;
    ma_uint32 encodedReadOffset;
    ma_uint32 encodedWriteOffset;
    ma_bool8 ownsBuffer;
    ma_bool8 clearOnWriteAcquire;
    ma_allocation_callbacks allocationCallbacks;
} ma_rb;

extern ma_result
ma_rb_init_ex(size_t subbufferSizeInBytes, size_t subbufferCount,
              size_t subbufferStrideInBytes, void *pOptionalPreallocatedBuffer,
              const ma_allocation_callbacks *pAllocationCallbacks, ma_rb *pRB);
extern ma_result ma_rb_init(size_t bufferSizeInBytes,
                            void *pOptionalPreallocatedBuffer,
                            const ma_allocation_callbacks *pAllocationCallbacks,
                            ma_rb *pRB);
extern void ma_rb_uninit(ma_rb *pRB);
extern void ma_rb_reset(ma_rb *pRB);
extern ma_result ma_rb_acquire_read(ma_rb *pRB, size_t *pSizeInBytes,
                                    void **ppBufferOut);
extern ma_result ma_rb_commit_read(ma_rb *pRB, size_t sizeInBytes,
                                   void *pBufferOut);
extern ma_result ma_rb_acquire_write(ma_rb *pRB, size_t *pSizeInBytes,
                                     void **ppBufferOut);
extern ma_result ma_rb_commit_write(ma_rb *pRB, size_t sizeInBytes,
                                    void *pBufferOut);
extern ma_result ma_rb_seek_read(ma_rb *pRB, size_t offsetInBytes);
extern ma_result ma_rb_seek_write(ma_rb *pRB, size_t offsetInBytes);
extern ma_int32 ma_rb_pointer_distance(ma_rb *pRB);
extern ma_uint32 ma_rb_available_read(ma_rb *pRB);
extern ma_uint32 ma_rb_available_write(ma_rb *pRB);
extern size_t ma_rb_get_subbuffer_size(ma_rb *pRB);
extern size_t ma_rb_get_subbuffer_stride(ma_rb *pRB);
extern size_t ma_rb_get_subbuffer_offset(ma_rb *pRB, size_t subbufferIndex);
extern void *ma_rb_get_subbuffer_ptr(ma_rb *pRB, size_t subbufferIndex,
                                     void *pBuffer);

typedef struct {
    ma_rb rb;
    ma_format format;
    ma_uint32 channels;
} ma_pcm_rb;

extern ma_result ma_pcm_rb_init_ex(
    ma_format format, ma_uint32 channels, ma_uint32 subbufferSizeInFrames,
    ma_uint32 subbufferCount, ma_uint32 subbufferStrideInFrames,
    void *pOptionalPreallocatedBuffer,
    const ma_allocation_callbacks *pAllocationCallbacks, ma_pcm_rb *pRB);
extern ma_result
ma_pcm_rb_init(ma_format format, ma_uint32 channels,
               ma_uint32 bufferSizeInFrames, void *pOptionalPreallocatedBuffer,
               const ma_allocation_callbacks *pAllocationCallbacks,
               ma_pcm_rb *pRB);
extern void ma_pcm_rb_uninit(ma_pcm_rb *pRB);
extern void ma_pcm_rb_reset(ma_pcm_rb *pRB);
extern ma_result ma_pcm_rb_acquire_read(ma_pcm_rb *pRB,
                                        ma_uint32 *pSizeInFrames,
                                        void **ppBufferOut);
extern ma_result ma_pcm_rb_commit_read(ma_pcm_rb *pRB, ma_uint32 sizeInFrames,
                                       void *pBufferOut);
extern ma_result ma_pcm_rb_acquire_write(ma_pcm_rb *pRB,
                                         ma_uint32 *pSizeInFrames,
                                         void **ppBufferOut);
extern ma_result ma_pcm_rb_commit_write(ma_pcm_rb *pRB, ma_uint32 sizeInFrames,
                                        void *pBufferOut);
extern ma_result ma_pcm_rb_seek_read(ma_pcm_rb *pRB, ma_uint32 offsetInFrames);
extern ma_result ma_pcm_rb_seek_write(ma_pcm_rb *pRB, ma_uint32 offsetInFrames);
extern ma_int32 ma_pcm_rb_pointer_distance(ma_pcm_rb *pRB);
extern ma_uint32 ma_pcm_rb_available_read(ma_pcm_rb *pRB);
extern ma_uint32 ma_pcm_rb_available_write(ma_pcm_rb *pRB);
extern ma_uint32 ma_pcm_rb_get_subbuffer_size(ma_pcm_rb *pRB);
extern ma_uint32 ma_pcm_rb_get_subbuffer_stride(ma_pcm_rb *pRB);
extern ma_uint32 ma_pcm_rb_get_subbuffer_offset(ma_pcm_rb *pRB,
                                                ma_uint32 subbufferIndex);
extern void *ma_pcm_rb_get_subbuffer_ptr(ma_pcm_rb *pRB,
                                         ma_uint32 subbufferIndex,
                                         void *pBuffer);

typedef struct {
    ma_pcm_rb rb;
} ma_duplex_rb;

extern ma_result
ma_duplex_rb_init(ma_format captureFormat, ma_uint32 captureChannels,
                  ma_uint32 sampleRate, ma_uint32 captureInternalSampleRate,
                  ma_uint32 captureInternalPeriodSizeInFrames,
                  const ma_allocation_callbacks *pAllocationCallbacks,
                  ma_duplex_rb *pRB);
extern ma_result ma_duplex_rb_uninit(ma_duplex_rb *pRB);

extern const char *ma_result_description(ma_result result);

extern void *ma_malloc(size_t sz,
                       const ma_allocation_callbacks *pAllocationCallbacks);

extern void *ma_realloc(void *p, size_t sz,
                        const ma_allocation_callbacks *pAllocationCallbacks);

extern void ma_free(void *p,
                    const ma_allocation_callbacks *pAllocationCallbacks);

extern void *
ma_aligned_malloc(size_t sz, size_t alignment,
                  const ma_allocation_callbacks *pAllocationCallbacks);

extern void
ma_aligned_free(void *p, const ma_allocation_callbacks *pAllocationCallbacks);

extern const char *ma_get_format_name(ma_format format);

extern void ma_blend_f32(float *pOut, float *pInA, float *pInB, float factor,
                         ma_uint32 channels);

extern ma_uint32 ma_get_bytes_per_sample(ma_format format);
static inline __attribute__((always_inline)) ma_uint32
ma_get_bytes_per_frame(ma_format format, ma_uint32 channels) {
    return ma_get_bytes_per_sample(format) * channels;
}

extern const char *ma_log_level_to_string(ma_uint32 logLevel);

typedef enum {
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
} ma_backend;

typedef void (*ma_device_callback_proc)(ma_device *pDevice, void *pOutput,
                                        const void *pInput,
                                        ma_uint32 frameCount);

typedef void (*ma_stop_proc)(ma_device *pDevice);

typedef void (*ma_log_proc)(ma_context *pContext, ma_device *pDevice,
                            ma_uint32 logLevel, const char *message);

typedef enum {
    ma_device_type_playback = 1,
    ma_device_type_capture = 2,
    ma_device_type_duplex = ma_device_type_playback | ma_device_type_capture,
    ma_device_type_loopback = 4
} ma_device_type;

typedef enum {
    ma_share_mode_shared = 0,
    ma_share_mode_exclusive
} ma_share_mode;

typedef enum {
    ma_ios_session_category_default = 0,
    ma_ios_session_category_none,
    ma_ios_session_category_ambient,
    ma_ios_session_category_solo_ambient,
    ma_ios_session_category_playback,
    ma_ios_session_category_record,
    ma_ios_session_category_play_and_record,
    ma_ios_session_category_multi_route
} ma_ios_session_category;

typedef enum {
    ma_ios_session_category_option_mix_with_others = 0x01,
    ma_ios_session_category_option_duck_others = 0x02,
    ma_ios_session_category_option_allow_bluetooth = 0x04,
    ma_ios_session_category_option_default_to_speaker = 0x08,
    ma_ios_session_category_option_interrupt_spoken_audio_and_mix_with_others =
        0x11,
    ma_ios_session_category_option_allow_bluetooth_a2dp = 0x20,
    ma_ios_session_category_option_allow_air_play = 0x40,
} ma_ios_session_category_option;

typedef enum {
    ma_opensl_stream_type_default = 0,
    ma_opensl_stream_type_voice,
    ma_opensl_stream_type_system,
    ma_opensl_stream_type_ring,
    ma_opensl_stream_type_media,
    ma_opensl_stream_type_alarm,
    ma_opensl_stream_type_notification
} ma_opensl_stream_type;

typedef enum {
    ma_opensl_recording_preset_default = 0,
    ma_opensl_recording_preset_generic,
    ma_opensl_recording_preset_camcorder,
    ma_opensl_recording_preset_voice_recognition,
    ma_opensl_recording_preset_voice_communication,
    ma_opensl_recording_preset_voice_unprocessed
} ma_opensl_recording_preset;

typedef enum {
    ma_aaudio_usage_default = 0,
    ma_aaudio_usage_announcement,
    ma_aaudio_usage_emergency,
    ma_aaudio_usage_safety,
    ma_aaudio_usage_vehicle_status,
    ma_aaudio_usage_alarm,
    ma_aaudio_usage_assistance_accessibility,
    ma_aaudio_usage_assistance_navigation_guidance,
    ma_aaudio_usage_assistance_sonification,
    ma_aaudio_usage_assitant,
    ma_aaudio_usage_game,
    ma_aaudio_usage_media,
    ma_aaudio_usage_notification,
    ma_aaudio_usage_notification_event,
    ma_aaudio_usage_notification_ringtone,
    ma_aaudio_usage_voice_communication,
    ma_aaudio_usage_voice_communication_signalling
} ma_aaudio_usage;

typedef enum {
    ma_aaudio_content_type_default = 0,
    ma_aaudio_content_type_movie,
    ma_aaudio_content_type_music,
    ma_aaudio_content_type_sonification,
    ma_aaudio_content_type_speech
} ma_aaudio_content_type;

typedef enum {
    ma_aaudio_input_preset_default = 0,
    ma_aaudio_input_preset_generic,
    ma_aaudio_input_preset_camcorder,
    ma_aaudio_input_preset_unprocessed,
    ma_aaudio_input_preset_voice_recognition,
    ma_aaudio_input_preset_voice_communication,
    ma_aaudio_input_preset_voice_performance
} ma_aaudio_input_preset;

typedef union {
    ma_int64 counter;
    double counterD;
} ma_timer;

typedef union {
    wchar_t wasapi[64];
    ma_uint8 dsound[16];
    ma_uint32 winmm;
    char alsa[256];
    char pulse[256];
    int jack;
    char coreaudio[256];
    char sndio[256];
    char audio4[256];
    char oss[64];
    ma_int32 aaudio;
    ma_uint32 opensl;
    char webaudio[32];
    union {
        int i;
        char s[256];
        void *p;
    } custom;
    int nullbackend;
} ma_device_id;

typedef struct ma_context_config ma_context_config;
typedef struct ma_device_config ma_device_config;
typedef struct ma_backend_callbacks ma_backend_callbacks;

typedef struct {

    ma_device_id id;
    char name[256];
    ma_bool32 isDefault;

    ma_uint32 formatCount;
    ma_format formats[ma_format_count];
    ma_uint32 minChannels;
    ma_uint32 maxChannels;
    ma_uint32 minSampleRate;
    ma_uint32 maxSampleRate;

    ma_uint32 nativeDataFormatCount;
    struct {
        ma_format format;
        ma_uint32 channels;
        ma_uint32 sampleRate;
        ma_uint32 flags;
    } nativeDataFormats[64];
} ma_device_info;

struct ma_device_config {
    ma_device_type deviceType;
    ma_uint32 sampleRate;
    ma_uint32 periodSizeInFrames;
    ma_uint32 periodSizeInMilliseconds;
    ma_uint32 periods;
    ma_performance_profile performanceProfile;
    ma_bool8 noPreZeroedOutputBuffer;
    ma_bool8 noClip;
    ma_device_callback_proc dataCallback;
    ma_stop_proc stopCallback;
    void *pUserData;
    struct {
        ma_resample_algorithm algorithm;
        struct {
            ma_uint32 lpfOrder;
        } linear;
        struct {
            int quality;
        } speex;
    } resampling;
    struct {
        const ma_device_id *pDeviceID;
        ma_format format;
        ma_uint32 channels;
        ma_channel channelMap[32];
        ma_channel_mix_mode channelMixMode;
        ma_share_mode shareMode;
    } playback;
    struct {
        const ma_device_id *pDeviceID;
        ma_format format;
        ma_uint32 channels;
        ma_channel channelMap[32];
        ma_channel_mix_mode channelMixMode;
        ma_share_mode shareMode;
    } capture;

    struct {
        ma_bool8 noAutoConvertSRC;
        ma_bool8 noDefaultQualitySRC;
        ma_bool8 noAutoStreamRouting;
        ma_bool8 noHardwareOffloading;
    } wasapi;
    struct {
        ma_bool32 noMMap;
        ma_bool32 noAutoFormat;
        ma_bool32 noAutoChannels;
        ma_bool32 noAutoResample;
    } alsa;
    struct {
        const char *pStreamNamePlayback;
        const char *pStreamNameCapture;
    } pulse;
    struct {
        ma_bool32 allowNominalSampleRateChange;
    } coreaudio;
    struct {
        ma_opensl_stream_type streamType;
        ma_opensl_recording_preset recordingPreset;
    } opensl;
    struct {
        ma_aaudio_usage usage;
        ma_aaudio_content_type contentType;
        ma_aaudio_input_preset inputPreset;
    } aaudio;
};

typedef ma_bool32 (*ma_enum_devices_callback_proc)(ma_context *pContext,
                                                   ma_device_type deviceType,
                                                   const ma_device_info *pInfo,
                                                   void *pUserData);

typedef struct {
    const ma_device_id *pDeviceID;
    ma_share_mode shareMode;
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    ma_channel channelMap[32];
    ma_uint32 periodSizeInFrames;
    ma_uint32 periodSizeInMilliseconds;
    ma_uint32 periodCount;
} ma_device_descriptor;

struct ma_backend_callbacks {
    ma_result (*onContextInit)(ma_context *pContext,
                               const ma_context_config *pConfig,
                               ma_backend_callbacks *pCallbacks);
    ma_result (*onContextUninit)(ma_context *pContext);
    ma_result (*onContextEnumerateDevices)(
        ma_context *pContext, ma_enum_devices_callback_proc callback,
        void *pUserData);
    ma_result (*onContextGetDeviceInfo)(ma_context *pContext,
                                        ma_device_type deviceType,
                                        const ma_device_id *pDeviceID,
                                        ma_device_info *pDeviceInfo);
    ma_result (*onDeviceInit)(ma_device *pDevice,
                              const ma_device_config *pConfig,
                              ma_device_descriptor *pDescriptorPlayback,
                              ma_device_descriptor *pDescriptorCapture);
    ma_result (*onDeviceUninit)(ma_device *pDevice);
    ma_result (*onDeviceStart)(ma_device *pDevice);
    ma_result (*onDeviceStop)(ma_device *pDevice);
    ma_result (*onDeviceRead)(ma_device *pDevice, void *pFrames,
                              ma_uint32 frameCount, ma_uint32 *pFramesRead);
    ma_result (*onDeviceWrite)(ma_device *pDevice, const void *pFrames,
                               ma_uint32 frameCount, ma_uint32 *pFramesWritten);
    ma_result (*onDeviceDataLoop)(ma_device *pDevice);
    ma_result (*onDeviceDataLoopWakeup)(ma_device *pDevice);
};

struct ma_context_config {
    ma_log_proc logCallback;
    ma_thread_priority threadPriority;
    size_t threadStackSize;
    void *pUserData;
    ma_allocation_callbacks allocationCallbacks;
    struct {
        ma_bool32 useVerboseDeviceEnumeration;
    } alsa;
    struct {
        const char *pApplicationName;
        const char *pServerName;
        ma_bool32 tryAutoSpawn;
    } pulse;
    struct {
        ma_ios_session_category sessionCategory;
        ma_uint32 sessionCategoryOptions;
        ma_bool32 noAudioSessionActivate;
        ma_bool32 noAudioSessionDeactivate;
    } coreaudio;
    struct {
        const char *pClientName;
        ma_bool32 tryStartServer;
    } jack;
    ma_backend_callbacks custom;
};

typedef struct {
    int code;
    ma_event *pEvent;
    union {
        struct {
            int _unused;
        } quit;
        struct {
            ma_device_type deviceType;
            void *pAudioClient;
            void **ppAudioClientService;
            ma_result *pResult;
        } createAudioClient;
        struct {
            ma_device *pDevice;
            ma_device_type deviceType;
        } releaseAudioClient;
    } data;
} ma_context_command__wasapi;

struct ma_context {
    ma_backend_callbacks callbacks;
    ma_backend backend;
    ma_log_proc logCallback;
    ma_thread_priority threadPriority;
    size_t threadStackSize;
    void *pUserData;
    ma_allocation_callbacks allocationCallbacks;
    ma_mutex deviceEnumLock;
    ma_mutex deviceInfoLock;
    ma_uint32 deviceInfoCapacity;
    ma_uint32 playbackDeviceInfoCount;
    ma_uint32 captureDeviceInfoCount;
    ma_device_info *pDeviceInfos;

    union {

        struct {
            ma_handle hCoreFoundation;
            ma_proc CFStringGetCString;
            ma_proc CFRelease;

            ma_handle hCoreAudio;
            ma_proc AudioObjectGetPropertyData;
            ma_proc AudioObjectGetPropertyDataSize;
            ma_proc AudioObjectSetPropertyData;
            ma_proc AudioObjectAddPropertyListener;
            ma_proc AudioObjectRemovePropertyListener;

            ma_handle hAudioUnit;
            ma_proc AudioComponentFindNext;
            ma_proc AudioComponentInstanceDispose;
            ma_proc AudioComponentInstanceNew;
            ma_proc AudioOutputUnitStart;
            ma_proc AudioOutputUnitStop;
            ma_proc AudioUnitAddPropertyListener;
            ma_proc AudioUnitGetPropertyInfo;
            ma_proc AudioUnitGetProperty;
            ma_proc AudioUnitSetProperty;
            ma_proc AudioUnitInitialize;
            ma_proc AudioUnitRender;

            ma_ptr component;
            ma_bool32 noAudioSessionDeactivate;
        } coreaudio;

        struct {
            int _unused;
        } null_backend;
    };

    union {

        struct {
            ma_handle pthreadSO;
            ma_proc pthread_create;
            ma_proc pthread_join;
            ma_proc pthread_mutex_init;
            ma_proc pthread_mutex_destroy;
            ma_proc pthread_mutex_lock;
            ma_proc pthread_mutex_unlock;
            ma_proc pthread_cond_init;
            ma_proc pthread_cond_destroy;
            ma_proc pthread_cond_wait;
            ma_proc pthread_cond_signal;
            ma_proc pthread_attr_init;
            ma_proc pthread_attr_destroy;
            ma_proc pthread_attr_setschedpolicy;
            ma_proc pthread_attr_getschedparam;
            ma_proc pthread_attr_setschedparam;
        } posix;

        int _unused;
    };
};

struct ma_device {
    ma_context *pContext;
    ma_device_type type;
    ma_uint32 sampleRate;
    ma_uint32 state;
    ma_device_callback_proc onData;
    ma_stop_proc onStop;
    void *pUserData;
    ma_mutex startStopLock;
    ma_event wakeupEvent;
    ma_event startEvent;
    ma_event stopEvent;
    ma_thread thread;
    ma_result workResult;
    ma_bool8 isOwnerOfContext;
    ma_bool8 noPreZeroedOutputBuffer;
    ma_bool8 noClip;
    float masterVolumeFactor;
    ma_duplex_rb duplexRB;
    struct {
        ma_resample_algorithm algorithm;
        struct {
            ma_uint32 lpfOrder;
        } linear;
        struct {
            int quality;
        } speex;
    } resampling;
    struct {
        ma_device_id id;
        char name[256];
        ma_share_mode shareMode;
        ma_format format;
        ma_uint32 channels;
        ma_channel channelMap[32];
        ma_format internalFormat;
        ma_uint32 internalChannels;
        ma_uint32 internalSampleRate;
        ma_channel internalChannelMap[32];
        ma_uint32 internalPeriodSizeInFrames;
        ma_uint32 internalPeriods;
        ma_channel_mix_mode channelMixMode;
        ma_data_converter converter;
    } playback;
    struct {
        ma_device_id id;
        char name[256];
        ma_share_mode shareMode;
        ma_format format;
        ma_uint32 channels;
        ma_channel channelMap[32];
        ma_format internalFormat;
        ma_uint32 internalChannels;
        ma_uint32 internalSampleRate;
        ma_channel internalChannelMap[32];
        ma_uint32 internalPeriodSizeInFrames;
        ma_uint32 internalPeriods;
        ma_channel_mix_mode channelMixMode;
        ma_data_converter converter;
    } capture;

    union {

        struct {
            ma_uint32 deviceObjectIDPlayback;
            ma_uint32 deviceObjectIDCapture;
            ma_ptr audioUnitPlayback;
            ma_ptr audioUnitCapture;
            ma_ptr pAudioBufferList;
            ma_uint32 audioBufferCapInFrames;
            ma_event stopEvent;
            ma_uint32 originalPeriodSizeInFrames;
            ma_uint32 originalPeriodSizeInMilliseconds;
            ma_uint32 originalPeriods;
            ma_performance_profile originalPerformanceProfile;
            ma_bool32 isDefaultPlaybackDevice;
            ma_bool32 isDefaultCaptureDevice;
            ma_bool32 isSwitchingPlaybackDevice;
            ma_bool32 isSwitchingCaptureDevice;
            void *pRouteChangeHandler;
        } coreaudio;

        struct {
            ma_thread deviceThread;
            ma_event operationEvent;
            ma_event operationCompletionEvent;
            ma_semaphore operationSemaphore;
            ma_uint32 operation;
            ma_result operationResult;
            ma_timer timer;
            double priorRunTime;
            ma_uint32 currentPeriodFramesRemainingPlayback;
            ma_uint32 currentPeriodFramesRemainingCapture;
            ma_uint64 lastProcessedFramePlayback;
            ma_uint64 lastProcessedFrameCapture;
            ma_bool32 isStarted;
        } null_device;
    };
};



extern ma_context_config ma_context_config_init(void);

extern ma_result ma_context_init(const ma_backend backends[],
                                 ma_uint32 backendCount,
                                 const ma_context_config *pConfig,
                                 ma_context *pContext);

extern ma_result ma_context_uninit(ma_context *pContext);

extern size_t ma_context_sizeof(void);

extern ma_result
ma_context_enumerate_devices(ma_context *pContext,
                             ma_enum_devices_callback_proc callback,
                             void *pUserData);

extern ma_result ma_context_get_devices(ma_context *pContext,
                                        ma_device_info **ppPlaybackDeviceInfos,
                                        ma_uint32 *pPlaybackDeviceCount,
                                        ma_device_info **ppCaptureDeviceInfos,
                                        ma_uint32 *pCaptureDeviceCount);

extern ma_result ma_context_get_device_info(ma_context *pContext,
                                            ma_device_type deviceType,
                                            const ma_device_id *pDeviceID,
                                            ma_share_mode shareMode,
                                            ma_device_info *pDeviceInfo);

extern ma_bool32 ma_context_is_loopback_supported(ma_context *pContext);

extern ma_device_config ma_device_config_init(ma_device_type deviceType);

extern ma_result ma_device_init(ma_context *pContext,
                                const ma_device_config *pConfig,
                                ma_device *pDevice);

extern ma_result ma_device_init_ex(const ma_backend backends[],
                                   ma_uint32 backendCount,
                                   const ma_context_config *pContextConfig,
                                   const ma_device_config *pConfig,
                                   ma_device *pDevice);

extern void ma_device_uninit(ma_device *pDevice);

extern ma_result ma_device_start(ma_device *pDevice);

extern ma_result ma_device_stop(ma_device *pDevice);

extern ma_bool32 ma_device_is_started(const ma_device *pDevice);

extern ma_uint32 ma_device_get_state(const ma_device *pDevice);

extern ma_result ma_device_set_master_volume(ma_device *pDevice, float volume);

extern ma_result ma_device_get_master_volume(ma_device *pDevice,
                                             float *pVolume);

extern ma_result ma_device_set_master_gain_db(ma_device *pDevice, float gainDB);

extern ma_result ma_device_get_master_gain_db(ma_device *pDevice,
                                              float *pGainDB);

extern ma_result ma_device_handle_backend_data_callback(ma_device *pDevice,
                                                        void *pOutput,
                                                        const void *pInput,
                                                        ma_uint32 frameCount);

extern ma_uint32 ma_calculate_buffer_size_in_frames_from_descriptor(
    const ma_device_descriptor *pDescriptor, ma_uint32 nativeSampleRate,
    ma_performance_profile performanceProfile);

extern const char *ma_get_backend_name(ma_backend backend);

extern ma_bool32 ma_is_backend_enabled(ma_backend backend);

extern ma_result ma_get_enabled_backends(ma_backend *pBackends,
                                         size_t backendCap,
                                         size_t *pBackendCount);

extern ma_bool32 ma_is_loopback_supported(ma_backend backend);

extern ma_result ma_spinlock_lock(volatile ma_spinlock *pSpinlock);

extern ma_result ma_spinlock_lock_noyield(volatile ma_spinlock *pSpinlock);

extern ma_result ma_spinlock_unlock(volatile ma_spinlock *pSpinlock);

extern ma_result ma_mutex_init(ma_mutex *pMutex);

extern void ma_mutex_uninit(ma_mutex *pMutex);

extern void ma_mutex_lock(ma_mutex *pMutex);

extern void ma_mutex_unlock(ma_mutex *pMutex);

extern ma_result ma_event_init(ma_event *pEvent);

extern void ma_event_uninit(ma_event *pEvent);

extern ma_result ma_event_wait(ma_event *pEvent);

extern ma_result ma_event_signal(ma_event *pEvent);

extern ma_uint32 ma_scale_buffer_size(ma_uint32 baseBufferSize, float scale);

extern ma_uint32 ma_calculate_buffer_size_in_milliseconds_from_frames(
    ma_uint32 bufferSizeInFrames, ma_uint32 sampleRate);

extern ma_uint32 ma_calculate_buffer_size_in_frames_from_milliseconds(
    ma_uint32 bufferSizeInMilliseconds, ma_uint32 sampleRate);

extern void ma_copy_pcm_frames(void *dst, const void *src, ma_uint64 frameCount,
                               ma_format format, ma_uint32 channels);

extern void ma_silence_pcm_frames(void *p, ma_uint64 frameCount,
                                  ma_format format, ma_uint32 channels);
static inline __attribute__((always_inline)) void
ma_zero_pcm_frames(void *p, ma_uint64 frameCount, ma_format format,
                   ma_uint32 channels) {
    ma_silence_pcm_frames(p, frameCount, format, channels);
}

extern void *ma_offset_pcm_frames_ptr(void *p, ma_uint64 offsetInFrames,
                                      ma_format format, ma_uint32 channels);
extern const void *ma_offset_pcm_frames_const_ptr(const void *p,
                                                  ma_uint64 offsetInFrames,
                                                  ma_format format,
                                                  ma_uint32 channels);
static inline __attribute__((always_inline)) float *
ma_offset_pcm_frames_ptr_f32(float *p, ma_uint64 offsetInFrames,
                             ma_uint32 channels) {
    return (float *)ma_offset_pcm_frames_ptr((void *)p, offsetInFrames,
                                             ma_format_f32, channels);
}
static inline __attribute__((always_inline)) const float *
ma_offset_pcm_frames_const_ptr_f32(const float *p, ma_uint64 offsetInFrames,
                                   ma_uint32 channels) {
    return (const float *)ma_offset_pcm_frames_const_ptr(
        (const void *)p, offsetInFrames, ma_format_f32, channels);
}

extern void ma_clip_samples_f32(float *p, ma_uint64 sampleCount);
static inline __attribute__((always_inline)) void
ma_clip_pcm_frames_f32(float *p, ma_uint64 frameCount, ma_uint32 channels) {
    ma_clip_samples_f32(p, frameCount * channels);
}

extern void ma_copy_and_apply_volume_factor_u8(ma_uint8 *pSamplesOut,
                                               const ma_uint8 *pSamplesIn,
                                               ma_uint64 sampleCount,
                                               float factor);
extern void ma_copy_and_apply_volume_factor_s16(ma_int16 *pSamplesOut,
                                                const ma_int16 *pSamplesIn,
                                                ma_uint64 sampleCount,
                                                float factor);
extern void ma_copy_and_apply_volume_factor_s24(void *pSamplesOut,
                                                const void *pSamplesIn,
                                                ma_uint64 sampleCount,
                                                float factor);
extern void ma_copy_and_apply_volume_factor_s32(ma_int32 *pSamplesOut,
                                                const ma_int32 *pSamplesIn,
                                                ma_uint64 sampleCount,
                                                float factor);
extern void ma_copy_and_apply_volume_factor_f32(float *pSamplesOut,
                                                const float *pSamplesIn,
                                                ma_uint64 sampleCount,
                                                float factor);

extern void ma_apply_volume_factor_u8(ma_uint8 *pSamples, ma_uint64 sampleCount,
                                      float factor);
extern void ma_apply_volume_factor_s16(ma_int16 *pSamples,
                                       ma_uint64 sampleCount, float factor);
extern void ma_apply_volume_factor_s24(void *pSamples, ma_uint64 sampleCount,
                                       float factor);
extern void ma_apply_volume_factor_s32(ma_int32 *pSamples,
                                       ma_uint64 sampleCount, float factor);
extern void ma_apply_volume_factor_f32(float *pSamples, ma_uint64 sampleCount,
                                       float factor);

extern void ma_copy_and_apply_volume_factor_pcm_frames_u8(
    ma_uint8 *pPCMFramesOut, const ma_uint8 *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor);
extern void ma_copy_and_apply_volume_factor_pcm_frames_s16(
    ma_int16 *pPCMFramesOut, const ma_int16 *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor);
extern void ma_copy_and_apply_volume_factor_pcm_frames_s24(
    void *pPCMFramesOut, const void *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor);
extern void ma_copy_and_apply_volume_factor_pcm_frames_s32(
    ma_int32 *pPCMFramesOut, const ma_int32 *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor);
extern void ma_copy_and_apply_volume_factor_pcm_frames_f32(
    float *pPCMFramesOut, const float *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor);
extern void ma_copy_and_apply_volume_factor_pcm_frames(
    void *pFramesOut, const void *pFramesIn, ma_uint64 frameCount,
    ma_format format, ma_uint32 channels, float factor);

extern void ma_apply_volume_factor_pcm_frames_u8(ma_uint8 *pFrames,
                                                 ma_uint64 frameCount,
                                                 ma_uint32 channels,
                                                 float factor);
extern void ma_apply_volume_factor_pcm_frames_s16(ma_int16 *pFrames,
                                                  ma_uint64 frameCount,
                                                  ma_uint32 channels,
                                                  float factor);
extern void ma_apply_volume_factor_pcm_frames_s24(void *pFrames,
                                                  ma_uint64 frameCount,
                                                  ma_uint32 channels,
                                                  float factor);
extern void ma_apply_volume_factor_pcm_frames_s32(ma_int32 *pFrames,
                                                  ma_uint64 frameCount,
                                                  ma_uint32 channels,
                                                  float factor);
extern void ma_apply_volume_factor_pcm_frames_f32(float *pFrames,
                                                  ma_uint64 frameCount,
                                                  ma_uint32 channels,
                                                  float factor);
extern void ma_apply_volume_factor_pcm_frames(void *pFrames,
                                              ma_uint64 frameCount,
                                              ma_format format,
                                              ma_uint32 channels, float factor);

extern float ma_factor_to_gain_db(float factor);

extern float ma_gain_db_to_factor(float gain);

typedef void ma_data_source;

typedef struct {
    ma_result (*onRead)(ma_data_source *pDataSource, void *pFramesOut,
                        ma_uint64 frameCount, ma_uint64 *pFramesRead);
    ma_result (*onSeek)(ma_data_source *pDataSource, ma_uint64 frameIndex);
    ma_result (*onMap)(ma_data_source *pDataSource, void **ppFramesOut,
                       ma_uint64 *pFrameCount);
    ma_result (*onUnmap)(ma_data_source *pDataSource, ma_uint64 frameCount);
    ma_result (*onGetDataFormat)(ma_data_source *pDataSource,
                                 ma_format *pFormat, ma_uint32 *pChannels,
                                 ma_uint32 *pSampleRate);
    ma_result (*onGetCursor)(ma_data_source *pDataSource, ma_uint64 *pCursor);
    ma_result (*onGetLength)(ma_data_source *pDataSource, ma_uint64 *pLength);
} ma_data_source_callbacks;

extern ma_result ma_data_source_read_pcm_frames(ma_data_source *pDataSource,
                                                void *pFramesOut,
                                                ma_uint64 frameCount,
                                                ma_uint64 *pFramesRead,
                                                ma_bool32 loop);
extern ma_result ma_data_source_seek_pcm_frames(ma_data_source *pDataSource,
                                                ma_uint64 frameCount,
                                                ma_uint64 *pFramesSeeked,
                                                ma_bool32 loop);
extern ma_result ma_data_source_seek_to_pcm_frame(ma_data_source *pDataSource,
                                                  ma_uint64 frameIndex);
extern ma_result ma_data_source_map(ma_data_source *pDataSource,
                                    void **ppFramesOut, ma_uint64 *pFrameCount);
extern ma_result ma_data_source_unmap(ma_data_source *pDataSource,
                                      ma_uint64 frameCount);
extern ma_result ma_data_source_get_data_format(ma_data_source *pDataSource,
                                                ma_format *pFormat,
                                                ma_uint32 *pChannels,
                                                ma_uint32 *pSampleRate);
extern ma_result
ma_data_source_get_cursor_in_pcm_frames(ma_data_source *pDataSource,
                                        ma_uint64 *pCursor);
extern ma_result
ma_data_source_get_length_in_pcm_frames(ma_data_source *pDataSource,
                                        ma_uint64 *pLength);

typedef struct {
    ma_data_source_callbacks ds;
    ma_format format;
    ma_uint32 channels;
    ma_uint64 cursor;
    ma_uint64 sizeInFrames;
    const void *pData;
} ma_audio_buffer_ref;

extern ma_result ma_audio_buffer_ref_init(ma_format format, ma_uint32 channels,
                                          const void *pData,
                                          ma_uint64 sizeInFrames,
                                          ma_audio_buffer_ref *pAudioBufferRef);
extern ma_result
ma_audio_buffer_ref_set_data(ma_audio_buffer_ref *pAudioBufferRef,
                             const void *pData, ma_uint64 sizeInFrames);
extern ma_uint64
ma_audio_buffer_ref_read_pcm_frames(ma_audio_buffer_ref *pAudioBufferRef,
                                    void *pFramesOut, ma_uint64 frameCount,
                                    ma_bool32 loop);
extern ma_result
ma_audio_buffer_ref_seek_to_pcm_frame(ma_audio_buffer_ref *pAudioBufferRef,
                                      ma_uint64 frameIndex);
extern ma_result ma_audio_buffer_ref_map(ma_audio_buffer_ref *pAudioBufferRef,
                                         void **ppFramesOut,
                                         ma_uint64 *pFrameCount);
extern ma_result ma_audio_buffer_ref_unmap(ma_audio_buffer_ref *pAudioBufferRef,
                                           ma_uint64 frameCount);
extern ma_result
ma_audio_buffer_ref_at_end(ma_audio_buffer_ref *pAudioBufferRef);
extern ma_result
ma_audio_buffer_ref_get_available_frames(ma_audio_buffer_ref *pAudioBufferRef,
                                         ma_uint64 *pAvailableFrames);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint64 sizeInFrames;
    const void *pData;
    ma_allocation_callbacks allocationCallbacks;
} ma_audio_buffer_config;

extern ma_audio_buffer_config ma_audio_buffer_config_init(
    ma_format format, ma_uint32 channels, ma_uint64 sizeInFrames,
    const void *pData, const ma_allocation_callbacks *pAllocationCallbacks);

typedef struct {
    ma_audio_buffer_ref ref;
    ma_allocation_callbacks allocationCallbacks;
    ma_bool32 ownsData;
    ma_uint8 _pExtraData[1];
} ma_audio_buffer;

extern ma_result ma_audio_buffer_init(const ma_audio_buffer_config *pConfig,
                                      ma_audio_buffer *pAudioBuffer);
extern ma_result
ma_audio_buffer_init_copy(const ma_audio_buffer_config *pConfig,
                          ma_audio_buffer *pAudioBuffer);
extern ma_result
ma_audio_buffer_alloc_and_init(const ma_audio_buffer_config *pConfig,
                               ma_audio_buffer **ppAudioBuffer);
extern void ma_audio_buffer_uninit(ma_audio_buffer *pAudioBuffer);
extern void ma_audio_buffer_uninit_and_free(ma_audio_buffer *pAudioBuffer);
extern ma_uint64 ma_audio_buffer_read_pcm_frames(ma_audio_buffer *pAudioBuffer,
                                                 void *pFramesOut,
                                                 ma_uint64 frameCount,
                                                 ma_bool32 loop);
extern ma_result
ma_audio_buffer_seek_to_pcm_frame(ma_audio_buffer *pAudioBuffer,
                                  ma_uint64 frameIndex);
extern ma_result ma_audio_buffer_map(ma_audio_buffer *pAudioBuffer,
                                     void **ppFramesOut,
                                     ma_uint64 *pFrameCount);
extern ma_result ma_audio_buffer_unmap(ma_audio_buffer *pAudioBuffer,
                                       ma_uint64 frameCount);
extern ma_result ma_audio_buffer_at_end(ma_audio_buffer *pAudioBuffer);
extern ma_result
ma_audio_buffer_get_available_frames(ma_audio_buffer *pAudioBuffer,
                                     ma_uint64 *pAvailableFrames);

typedef void ma_vfs;
typedef ma_handle ma_vfs_file;

typedef enum {
    ma_seek_origin_start,
    ma_seek_origin_current,
    ma_seek_origin_end
} ma_seek_origin;

typedef struct {
    ma_uint64 sizeInBytes;
} ma_file_info;

typedef struct {
    ma_result (*onOpen)(ma_vfs *pVFS, const char *pFilePath, ma_uint32 openMode,
                        ma_vfs_file *pFile);
    ma_result (*onOpenW)(ma_vfs *pVFS, const wchar_t *pFilePath,
                         ma_uint32 openMode, ma_vfs_file *pFile);
    ma_result (*onClose)(ma_vfs *pVFS, ma_vfs_file file);
    ma_result (*onRead)(ma_vfs *pVFS, ma_vfs_file file, void *pDst,
                        size_t sizeInBytes, size_t *pBytesRead);
    ma_result (*onWrite)(ma_vfs *pVFS, ma_vfs_file file, const void *pSrc,
                         size_t sizeInBytes, size_t *pBytesWritten);
    ma_result (*onSeek)(ma_vfs *pVFS, ma_vfs_file file, ma_int64 offset,
                        ma_seek_origin origin);
    ma_result (*onTell)(ma_vfs *pVFS, ma_vfs_file file, ma_int64 *pCursor);
    ma_result (*onInfo)(ma_vfs *pVFS, ma_vfs_file file, ma_file_info *pInfo);
} ma_vfs_callbacks;

extern ma_result ma_vfs_open(ma_vfs *pVFS, const char *pFilePath,
                             ma_uint32 openMode, ma_vfs_file *pFile);
extern ma_result ma_vfs_open_w(ma_vfs *pVFS, const wchar_t *pFilePath,
                               ma_uint32 openMode, ma_vfs_file *pFile);
extern ma_result ma_vfs_close(ma_vfs *pVFS, ma_vfs_file file);
extern ma_result ma_vfs_read(ma_vfs *pVFS, ma_vfs_file file, void *pDst,
                             size_t sizeInBytes, size_t *pBytesRead);
extern ma_result ma_vfs_write(ma_vfs *pVFS, ma_vfs_file file, const void *pSrc,
                              size_t sizeInBytes, size_t *pBytesWritten);
extern ma_result ma_vfs_seek(ma_vfs *pVFS, ma_vfs_file file, ma_int64 offset,
                             ma_seek_origin origin);
extern ma_result ma_vfs_tell(ma_vfs *pVFS, ma_vfs_file file, ma_int64 *pCursor);
extern ma_result ma_vfs_info(ma_vfs *pVFS, ma_vfs_file file,
                             ma_file_info *pInfo);
extern ma_result
ma_vfs_open_and_read_file(ma_vfs *pVFS, const char *pFilePath, void **ppData,
                          size_t *pSize,
                          const ma_allocation_callbacks *pAllocationCallbacks);

typedef struct {
    ma_vfs_callbacks cb;
    ma_allocation_callbacks allocationCallbacks;
} ma_default_vfs;

extern ma_result
ma_default_vfs_init(ma_default_vfs *pVFS,
                    const ma_allocation_callbacks *pAllocationCallbacks);

typedef enum { ma_resource_format_wav } ma_resource_format;

typedef struct ma_decoder ma_decoder;

typedef size_t (*ma_decoder_read_proc)(ma_decoder *pDecoder, void *pBufferOut,
                                       size_t bytesToRead);
typedef ma_bool32 (*ma_decoder_seek_proc)(ma_decoder *pDecoder, int byteOffset,
                                          ma_seek_origin origin);
typedef ma_uint64 (*ma_decoder_read_pcm_frames_proc)(ma_decoder *pDecoder,
                                                     void *pFramesOut,
                                                     ma_uint64 frameCount);
typedef ma_result (*ma_decoder_seek_to_pcm_frame_proc)(ma_decoder *pDecoder,
                                                       ma_uint64 frameIndex);
typedef ma_result (*ma_decoder_uninit_proc)(ma_decoder *pDecoder);
typedef ma_uint64 (*ma_decoder_get_length_in_pcm_frames_proc)(
    ma_decoder *pDecoder);

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    ma_channel channelMap[32];
    ma_channel_mix_mode channelMixMode;
    ma_dither_mode ditherMode;
    struct {
        ma_resample_algorithm algorithm;
        struct {
            ma_uint32 lpfOrder;
        } linear;
        struct {
            int quality;
        } speex;
    } resampling;
    ma_allocation_callbacks allocationCallbacks;
} ma_decoder_config;

struct ma_decoder {
    ma_data_source_callbacks ds;
    ma_decoder_read_proc onRead;
    ma_decoder_seek_proc onSeek;
    void *pUserData;
    ma_uint64 readPointerInBytes;
    ma_uint64 readPointerInPCMFrames;
    ma_format internalFormat;
    ma_uint32 internalChannels;
    ma_uint32 internalSampleRate;
    ma_channel internalChannelMap[32];
    ma_format outputFormat;
    ma_uint32 outputChannels;
    ma_uint32 outputSampleRate;
    ma_channel outputChannelMap[32];
    ma_data_converter converter;
    ma_allocation_callbacks allocationCallbacks;
    ma_decoder_read_pcm_frames_proc onReadPCMFrames;
    ma_decoder_seek_to_pcm_frame_proc onSeekToPCMFrame;
    ma_decoder_uninit_proc onUninit;
    ma_decoder_get_length_in_pcm_frames_proc onGetLengthInPCMFrames;
    void *pInternalDecoder;
    union {
        struct {
            ma_vfs *pVFS;
            ma_vfs_file file;
        } vfs;
        struct {
            const ma_uint8 *pData;
            size_t dataSize;
            size_t currentReadPos;
        } memory;
    } backend;
};

extern ma_decoder_config ma_decoder_config_init(ma_format outputFormat,
                                                ma_uint32 outputChannels,
                                                ma_uint32 outputSampleRate);

extern ma_result ma_decoder_init(ma_decoder_read_proc onRead,
                                 ma_decoder_seek_proc onSeek, void *pUserData,
                                 const ma_decoder_config *pConfig,
                                 ma_decoder *pDecoder);
extern ma_result ma_decoder_init_wav(ma_decoder_read_proc onRead,
                                     ma_decoder_seek_proc onSeek,
                                     void *pUserData,
                                     const ma_decoder_config *pConfig,
                                     ma_decoder *pDecoder);
extern ma_result ma_decoder_init_flac(ma_decoder_read_proc onRead,
                                      ma_decoder_seek_proc onSeek,
                                      void *pUserData,
                                      const ma_decoder_config *pConfig,
                                      ma_decoder *pDecoder);
extern ma_result ma_decoder_init_mp3(ma_decoder_read_proc onRead,
                                     ma_decoder_seek_proc onSeek,
                                     void *pUserData,
                                     const ma_decoder_config *pConfig,
                                     ma_decoder *pDecoder);
extern ma_result ma_decoder_init_vorbis(ma_decoder_read_proc onRead,
                                        ma_decoder_seek_proc onSeek,
                                        void *pUserData,
                                        const ma_decoder_config *pConfig,
                                        ma_decoder *pDecoder);
extern ma_result
ma_decoder_init_raw(ma_decoder_read_proc onRead, ma_decoder_seek_proc onSeek,
                    void *pUserData, const ma_decoder_config *pConfigIn,
                    const ma_decoder_config *pConfigOut, ma_decoder *pDecoder);

extern ma_result ma_decoder_init_memory(const void *pData, size_t dataSize,
                                        const ma_decoder_config *pConfig,
                                        ma_decoder *pDecoder);
extern ma_result ma_decoder_init_memory_wav(const void *pData, size_t dataSize,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder);
extern ma_result ma_decoder_init_memory_flac(const void *pData, size_t dataSize,
                                             const ma_decoder_config *pConfig,
                                             ma_decoder *pDecoder);
extern ma_result ma_decoder_init_memory_mp3(const void *pData, size_t dataSize,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder);
extern ma_result ma_decoder_init_memory_vorbis(const void *pData,
                                               size_t dataSize,
                                               const ma_decoder_config *pConfig,
                                               ma_decoder *pDecoder);
extern ma_result ma_decoder_init_memory_raw(const void *pData, size_t dataSize,
                                            const ma_decoder_config *pConfigIn,
                                            const ma_decoder_config *pConfigOut,
                                            ma_decoder *pDecoder);

extern ma_result ma_decoder_init_vfs(ma_vfs *pVFS, const char *pFilePath,
                                     const ma_decoder_config *pConfig,
                                     ma_decoder *pDecoder);
extern ma_result ma_decoder_init_vfs_wav(ma_vfs *pVFS, const char *pFilePath,
                                         const ma_decoder_config *pConfig,
                                         ma_decoder *pDecoder);
extern ma_result ma_decoder_init_vfs_flac(ma_vfs *pVFS, const char *pFilePath,
                                          const ma_decoder_config *pConfig,
                                          ma_decoder *pDecoder);
extern ma_result ma_decoder_init_vfs_mp3(ma_vfs *pVFS, const char *pFilePath,
                                         const ma_decoder_config *pConfig,
                                         ma_decoder *pDecoder);
extern ma_result ma_decoder_init_vfs_vorbis(ma_vfs *pVFS, const char *pFilePath,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder);

extern ma_result ma_decoder_init_vfs_w(ma_vfs *pVFS, const wchar_t *pFilePath,
                                       const ma_decoder_config *pConfig,
                                       ma_decoder *pDecoder);
extern ma_result ma_decoder_init_vfs_wav_w(ma_vfs *pVFS,
                                           const wchar_t *pFilePath,
                                           const ma_decoder_config *pConfig,
                                           ma_decoder *pDecoder);
extern ma_result ma_decoder_init_vfs_flac_w(ma_vfs *pVFS,
                                            const wchar_t *pFilePath,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder);
extern ma_result ma_decoder_init_vfs_mp3_w(ma_vfs *pVFS,
                                           const wchar_t *pFilePath,
                                           const ma_decoder_config *pConfig,
                                           ma_decoder *pDecoder);
extern ma_result ma_decoder_init_vfs_vorbis_w(ma_vfs *pVFS,
                                              const wchar_t *pFilePath,
                                              const ma_decoder_config *pConfig,
                                              ma_decoder *pDecoder);

extern ma_result ma_decoder_init_file(const char *pFilePath,
                                      const ma_decoder_config *pConfig,
                                      ma_decoder *pDecoder);
extern ma_result ma_decoder_init_file_wav(const char *pFilePath,
                                          const ma_decoder_config *pConfig,
                                          ma_decoder *pDecoder);
extern ma_result ma_decoder_init_file_flac(const char *pFilePath,
                                           const ma_decoder_config *pConfig,
                                           ma_decoder *pDecoder);
extern ma_result ma_decoder_init_file_mp3(const char *pFilePath,
                                          const ma_decoder_config *pConfig,
                                          ma_decoder *pDecoder);
extern ma_result ma_decoder_init_file_vorbis(const char *pFilePath,
                                             const ma_decoder_config *pConfig,
                                             ma_decoder *pDecoder);

extern ma_result ma_decoder_init_file_w(const wchar_t *pFilePath,
                                        const ma_decoder_config *pConfig,
                                        ma_decoder *pDecoder);
extern ma_result ma_decoder_init_file_wav_w(const wchar_t *pFilePath,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder);
extern ma_result ma_decoder_init_file_flac_w(const wchar_t *pFilePath,
                                             const ma_decoder_config *pConfig,
                                             ma_decoder *pDecoder);
extern ma_result ma_decoder_init_file_mp3_w(const wchar_t *pFilePath,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder);
extern ma_result ma_decoder_init_file_vorbis_w(const wchar_t *pFilePath,
                                               const ma_decoder_config *pConfig,
                                               ma_decoder *pDecoder);

extern ma_result ma_decoder_uninit(ma_decoder *pDecoder);

extern ma_result ma_decoder_get_cursor_in_pcm_frames(ma_decoder *pDecoder,
                                                     ma_uint64 *pCursor);

extern ma_uint64 ma_decoder_get_length_in_pcm_frames(ma_decoder *pDecoder);

extern ma_uint64 ma_decoder_read_pcm_frames(ma_decoder *pDecoder,
                                            void *pFramesOut,
                                            ma_uint64 frameCount);

extern ma_result ma_decoder_seek_to_pcm_frame(ma_decoder *pDecoder,
                                              ma_uint64 frameIndex);

extern ma_result ma_decoder_get_available_frames(ma_decoder *pDecoder,
                                                 ma_uint64 *pAvailableFrames);

extern ma_result ma_decode_from_vfs(ma_vfs *pVFS, const char *pFilePath,
                                    ma_decoder_config *pConfig,
                                    ma_uint64 *pFrameCountOut,
                                    void **ppPCMFramesOut);
extern ma_result ma_decode_file(const char *pFilePath,
                                ma_decoder_config *pConfig,
                                ma_uint64 *pFrameCountOut,
                                void **ppPCMFramesOut);
extern ma_result ma_decode_memory(const void *pData, size_t dataSize,
                                  ma_decoder_config *pConfig,
                                  ma_uint64 *pFrameCountOut,
                                  void **ppPCMFramesOut);

typedef struct ma_encoder ma_encoder;

typedef size_t (*ma_encoder_write_proc)(ma_encoder *pEncoder,
                                        const void *pBufferIn,
                                        size_t bytesToWrite);
typedef ma_bool32 (*ma_encoder_seek_proc)(ma_encoder *pEncoder, int byteOffset,
                                          ma_seek_origin origin);
typedef ma_result (*ma_encoder_init_proc)(ma_encoder *pEncoder);
typedef void (*ma_encoder_uninit_proc)(ma_encoder *pEncoder);
typedef ma_uint64 (*ma_encoder_write_pcm_frames_proc)(ma_encoder *pEncoder,
                                                      const void *pFramesIn,
                                                      ma_uint64 frameCount);

typedef struct {
    ma_resource_format resourceFormat;
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    ma_allocation_callbacks allocationCallbacks;
} ma_encoder_config;

extern ma_encoder_config
ma_encoder_config_init(ma_resource_format resourceFormat, ma_format format,
                       ma_uint32 channels, ma_uint32 sampleRate);

struct ma_encoder {
    ma_encoder_config config;
    ma_encoder_write_proc onWrite;
    ma_encoder_seek_proc onSeek;
    ma_encoder_init_proc onInit;
    ma_encoder_uninit_proc onUninit;
    ma_encoder_write_pcm_frames_proc onWritePCMFrames;
    void *pUserData;
    void *pInternalEncoder;
    void *pFile;
};

extern ma_result ma_encoder_init(ma_encoder_write_proc onWrite,
                                 ma_encoder_seek_proc onSeek, void *pUserData,
                                 const ma_encoder_config *pConfig,
                                 ma_encoder *pEncoder);
extern ma_result ma_encoder_init_file(const char *pFilePath,
                                      const ma_encoder_config *pConfig,
                                      ma_encoder *pEncoder);
extern ma_result ma_encoder_init_file_w(const wchar_t *pFilePath,
                                        const ma_encoder_config *pConfig,
                                        ma_encoder *pEncoder);
extern void ma_encoder_uninit(ma_encoder *pEncoder);
extern ma_uint64 ma_encoder_write_pcm_frames(ma_encoder *pEncoder,
                                             const void *pFramesIn,
                                             ma_uint64 frameCount);

typedef enum {
    ma_waveform_type_sine,
    ma_waveform_type_square,
    ma_waveform_type_triangle,
    ma_waveform_type_sawtooth
} ma_waveform_type;

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_uint32 sampleRate;
    ma_waveform_type type;
    double amplitude;
    double frequency;
} ma_waveform_config;

extern ma_waveform_config
ma_waveform_config_init(ma_format format, ma_uint32 channels,
                        ma_uint32 sampleRate, ma_waveform_type type,
                        double amplitude, double frequency);

typedef struct {
    ma_data_source_callbacks ds;
    ma_waveform_config config;
    double advance;
    double time;
} ma_waveform;

extern ma_result ma_waveform_init(const ma_waveform_config *pConfig,
                                  ma_waveform *pWaveform);
extern ma_uint64 ma_waveform_read_pcm_frames(ma_waveform *pWaveform,
                                             void *pFramesOut,
                                             ma_uint64 frameCount);
extern ma_result ma_waveform_seek_to_pcm_frame(ma_waveform *pWaveform,
                                               ma_uint64 frameIndex);
extern ma_result ma_waveform_set_amplitude(ma_waveform *pWaveform,
                                           double amplitude);
extern ma_result ma_waveform_set_frequency(ma_waveform *pWaveform,
                                           double frequency);
extern ma_result ma_waveform_set_type(ma_waveform *pWaveform,
                                      ma_waveform_type type);
extern ma_result ma_waveform_set_sample_rate(ma_waveform *pWaveform,
                                             ma_uint32 sampleRate);

typedef enum {
    ma_noise_type_white,
    ma_noise_type_pink,
    ma_noise_type_brownian
} ma_noise_type;

typedef struct {
    ma_format format;
    ma_uint32 channels;
    ma_noise_type type;
    ma_int32 seed;
    double amplitude;
    ma_bool32 duplicateChannels;
} ma_noise_config;

extern ma_noise_config ma_noise_config_init(ma_format format,
                                            ma_uint32 channels,
                                            ma_noise_type type, ma_int32 seed,
                                            double amplitude);

typedef struct {
    ma_data_source_callbacks ds;
    ma_noise_config config;
    ma_lcg lcg;
    union {
        struct {
            double bin[32][16];
            double accumulation[32];
            ma_uint32 counter[32];
        } pink;
        struct {
            double accumulation[32];
        } brownian;
    } state;
} ma_noise;

extern ma_result ma_noise_init(const ma_noise_config *pConfig,
                               ma_noise *pNoise);
extern ma_uint64 ma_noise_read_pcm_frames(ma_noise *pNoise, void *pFramesOut,
                                          ma_uint64 frameCount);
extern ma_result ma_noise_set_amplitude(ma_noise *pNoise, double amplitude);
extern ma_result ma_noise_set_seed(ma_noise *pNoise, ma_int32 seed);
extern ma_result ma_noise_set_type(ma_noise *pNoise, ma_noise_type type);

