



struct ma_context {
    ma_backend_callbacks callbacks
    ma_backend backend
    ma_log_proc logCallback
    ma_thread_priority threadPriority
    size_t threadStackSize
    void *pUserData
    ma_allocation_callbacks allocationCallbacks
    ma_mutex deviceEnumLock
    ma_mutex deviceInfoLock
    ma_uint32 deviceInfoCapacity
    ma_uint32 playbackDeviceInfoCount
    ma_uint32 captureDeviceInfoCount
    ma_device_info *pDeviceInfos

    union {

        struct {
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

        struct {
            int _unused
        } null_backend
    }

    union {

        struct {
            ma_handle pthreadSO
            ma_proc pthread_create
            ma_proc pthread_join
            ma_proc pthread_mutex_init
            ma_proc pthread_mutex_destroy
            ma_proc pthread_mutex_lock
            ma_proc pthread_mutex_unlock
            ma_proc pthread_cond_init
            ma_proc pthread_cond_destroy
            ma_proc pthread_cond_wait
            ma_proc pthread_cond_signal
            ma_proc pthread_attr_init
            ma_proc pthread_attr_destroy
            ma_proc pthread_attr_setschedpolicy
            ma_proc pthread_attr_getschedparam
            ma_proc pthread_attr_setschedparam
        } posix

        int _unused
    }
}

struct ma_device {
    ma_context *pContext
    ma_device_type type
    ma_uint32 sampleRate
    ma_uint32 state
    ma_device_callback_proc onData
    ma_stop_proc onStop
    void *pUserData
    ma_mutex startStopLock
    ma_event wakeupEvent
    ma_event startEvent
    ma_event stopEvent
    ma_thread thread
    ma_result workResult
    ma_bool8 isOwnerOfContext
    ma_bool8 noPreZeroedOutputBuffer
    ma_bool8 noClip
    float masterVolumeFactor
    ma_duplex_rb duplexRB
    struct {
        ma_resample_algorithm algorithm
        struct {
            ma_uint32 lpfOrder
        } linear
        struct {
            int quality
        } speex
    } resampling
    struct {
        ma_device_id id
        char name[256]
        ma_share_mode shareMode
        ma_format format
        ma_uint32 channels
        ma_channel channelMap[32]
        ma_format internalFormat
        ma_uint32 internalChannels
        ma_uint32 internalSampleRate
        ma_channel internalChannelMap[32]
        ma_uint32 internalPeriodSizeInFrames
        ma_uint32 internalPeriods
        ma_channel_mix_mode channelMixMode
        ma_data_converter converter
    } playback
    struct {
        ma_device_id id
        char name[256]
        ma_share_mode shareMode
        ma_format format
        ma_uint32 channels
        ma_channel channelMap[32]
        ma_format internalFormat
        ma_uint32 internalChannels
        ma_uint32 internalSampleRate
        ma_channel internalChannelMap[32]
        ma_uint32 internalPeriodSizeInFrames
        ma_uint32 internalPeriods
        ma_channel_mix_mode channelMixMode
        ma_data_converter converter
    } capture

    union {

        struct {
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
            void *pRouteChangeHandler
        } coreaudio

        struct {
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
            ma_bool32 isStarted
        } null_device
    }
}



ma_context_config ma_context_config_init(void)

ma_result ma_context_init(const ma_backend backends[],
                                 ma_uint32 backendCount,
                                 const ma_context_config *pConfig,
                                 ma_context *pContext)

ma_result ma_context_uninit(ma_context *pContext)

size_t ma_context_sizeof(void)

ma_result
ma_context_enumerate_devices(ma_context *pContext,
                             ma_enum_devices_callback_proc callback,
                             void *pUserData)

ma_result ma_context_get_devices(ma_context *pContext,
                                        ma_device_info **ppPlaybackDeviceInfos,
                                        ma_uint32 *pPlaybackDeviceCount,
                                        ma_device_info **ppCaptureDeviceInfos,
                                        ma_uint32 *pCaptureDeviceCount)

ma_result ma_context_get_device_info(ma_context *pContext,
                                            ma_device_type deviceType,
                                            const ma_device_id *pDeviceID,
                                            ma_share_mode shareMode,
                                            ma_device_info *pDeviceInfo)

ma_bool32 ma_context_is_loopback_supported(ma_context *pContext)

ma_device_config ma_device_config_init(ma_device_type deviceType)

ma_result ma_device_init(ma_context *pContext,
                                const ma_device_config *pConfig,
                                ma_device *pDevice)

ma_result ma_device_init_ex(const ma_backend backends[],
                                   ma_uint32 backendCount,
                                   const ma_context_config *pContextConfig,
                                   const ma_device_config *pConfig,
                                   ma_device *pDevice)

void ma_device_uninit(ma_device *pDevice)

ma_result ma_device_start(ma_device *pDevice)

ma_result ma_device_stop(ma_device *pDevice)

ma_bool32 ma_device_is_started(const ma_device *pDevice)

ma_uint32 ma_device_get_state(const ma_device *pDevice)

ma_result ma_device_set_master_volume(ma_device *pDevice, float volume)

ma_result ma_device_get_master_volume(ma_device *pDevice,
                                             float *pVolume)

ma_result ma_device_set_master_gain_db(ma_device *pDevice, float gainDB)

ma_result ma_device_get_master_gain_db(ma_device *pDevice,
                                              float *pGainDB)

ma_result ma_device_handle_backend_data_callback(ma_device *pDevice,
                                                        void *pOutput,
                                                        const void *pInput,
                                                        ma_uint32 frameCount)

ma_uint32 ma_calculate_buffer_size_in_frames_from_descriptor(
    const ma_device_descriptor *pDescriptor, ma_uint32 nativeSampleRate,
    ma_performance_profile performanceProfile)

const char *ma_get_backend_name(ma_backend backend)

ma_bool32 ma_is_backend_enabled(ma_backend backend)

ma_result ma_get_enabled_backends(ma_backend *pBackends,
                                         size_t backendCap,
                                         size_t *pBackendCount)

ma_bool32 ma_is_loopback_supported(ma_backend backend)

ma_result ma_spinlock_lock(volatile ma_spinlock *pSpinlock)

ma_result ma_spinlock_lock_noyield(volatile ma_spinlock *pSpinlock)

ma_result ma_spinlock_unlock(volatile ma_spinlock *pSpinlock)

ma_result ma_mutex_init(ma_mutex *pMutex)

void ma_mutex_uninit(ma_mutex *pMutex)

void ma_mutex_lock(ma_mutex *pMutex)

void ma_mutex_unlock(ma_mutex *pMutex)

ma_result ma_event_init(ma_event *pEvent)

void ma_event_uninit(ma_event *pEvent)

ma_result ma_event_wait(ma_event *pEvent)

ma_result ma_event_signal(ma_event *pEvent)

ma_uint32 ma_scale_buffer_size(ma_uint32 baseBufferSize, float scale)

ma_uint32 ma_calculate_buffer_size_in_milliseconds_from_frames(
    ma_uint32 bufferSizeInFrames, ma_uint32 sampleRate)

ma_uint32 ma_calculate_buffer_size_in_frames_from_milliseconds(
    ma_uint32 bufferSizeInMilliseconds, ma_uint32 sampleRate)

void ma_copy_pcm_frames(void *dst, const void *src, ma_uint64 frameCount,
                               ma_format format, ma_uint32 channels)

void ma_silence_pcm_frames(void *p, ma_uint64 frameCount,
                                  ma_format format, ma_uint32 channels)
static inline __attribute__((always_inline)) void
ma_zero_pcm_frames(void *p, ma_uint64 frameCount, ma_format format,
                   ma_uint32 channels) {
    ma_silence_pcm_frames(p, frameCount, format, channels)
}

void *ma_offset_pcm_frames_ptr(void *p, ma_uint64 offsetInFrames,
                                      ma_format format, ma_uint32 channels)
const void *ma_offset_pcm_frames_const_ptr(const void *p,
                                                  ma_uint64 offsetInFrames,
                                                  ma_format format,
                                                  ma_uint32 channels)
static inline __attribute__((always_inline)) float *
ma_offset_pcm_frames_ptr_f32(float *p, ma_uint64 offsetInFrames,
                             ma_uint32 channels) {
    return (float *)ma_offset_pcm_frames_ptr((void *)p, offsetInFrames,
                                             ma_format_f32, channels)
}
static inline __attribute__((always_inline)) const float *
ma_offset_pcm_frames_const_ptr_f32(const float *p, ma_uint64 offsetInFrames,
                                   ma_uint32 channels) {
    return (const float *)ma_offset_pcm_frames_const_ptr(
        (const void *)p, offsetInFrames, ma_format_f32, channels)
}

void ma_clip_samples_f32(float *p, ma_uint64 sampleCount)
static inline __attribute__((always_inline)) void
ma_clip_pcm_frames_f32(float *p, ma_uint64 frameCount, ma_uint32 channels) {
    ma_clip_samples_f32(p, frameCount * channels)
}

void ma_copy_and_apply_volume_factor_u8(ma_uint8 *pSamplesOut,
                                               const ma_uint8 *pSamplesIn,
                                               ma_uint64 sampleCount,
                                               float factor)
void ma_copy_and_apply_volume_factor_s16(ma_int16 *pSamplesOut,
                                                const ma_int16 *pSamplesIn,
                                                ma_uint64 sampleCount,
                                                float factor)
void ma_copy_and_apply_volume_factor_s24(void *pSamplesOut,
                                                const void *pSamplesIn,
                                                ma_uint64 sampleCount,
                                                float factor)
void ma_copy_and_apply_volume_factor_s32(ma_int32 *pSamplesOut,
                                                const ma_int32 *pSamplesIn,
                                                ma_uint64 sampleCount,
                                                float factor)
void ma_copy_and_apply_volume_factor_f32(float *pSamplesOut,
                                                const float *pSamplesIn,
                                                ma_uint64 sampleCount,
                                                float factor)

void ma_apply_volume_factor_u8(ma_uint8 *pSamples, ma_uint64 sampleCount,
                                      float factor)
void ma_apply_volume_factor_s16(ma_int16 *pSamples,
                                       ma_uint64 sampleCount, float factor)
void ma_apply_volume_factor_s24(void *pSamples, ma_uint64 sampleCount,
                                       float factor)
void ma_apply_volume_factor_s32(ma_int32 *pSamples,
                                       ma_uint64 sampleCount, float factor)
void ma_apply_volume_factor_f32(float *pSamples, ma_uint64 sampleCount,
                                       float factor)

void ma_copy_and_apply_volume_factor_pcm_frames_u8(
    ma_uint8 *pPCMFramesOut, const ma_uint8 *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames_s16(
    ma_int16 *pPCMFramesOut, const ma_int16 *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames_s24(
    void *pPCMFramesOut, const void *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames_s32(
    ma_int32 *pPCMFramesOut, const ma_int32 *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames_f32(
    float *pPCMFramesOut, const float *pPCMFramesIn, ma_uint64 frameCount,
    ma_uint32 channels, float factor)
void ma_copy_and_apply_volume_factor_pcm_frames(
    void *pFramesOut, const void *pFramesIn, ma_uint64 frameCount,
    ma_format format, ma_uint32 channels, float factor)

void ma_apply_volume_factor_pcm_frames_u8(ma_uint8 *pFrames,
                                                 ma_uint64 frameCount,
                                                 ma_uint32 channels,
                                                 float factor)
void ma_apply_volume_factor_pcm_frames_s16(ma_int16 *pFrames,
                                                  ma_uint64 frameCount,
                                                  ma_uint32 channels,
                                                  float factor)
void ma_apply_volume_factor_pcm_frames_s24(void *pFrames,
                                                  ma_uint64 frameCount,
                                                  ma_uint32 channels,
                                                  float factor)
void ma_apply_volume_factor_pcm_frames_s32(ma_int32 *pFrames,
                                                  ma_uint64 frameCount,
                                                  ma_uint32 channels,
                                                  float factor)
void ma_apply_volume_factor_pcm_frames_f32(float *pFrames,
                                                  ma_uint64 frameCount,
                                                  ma_uint32 channels,
                                                  float factor)
void ma_apply_volume_factor_pcm_frames(void *pFrames,
                                              ma_uint64 frameCount,
                                              ma_format format,
                                              ma_uint32 channels, float factor)

float ma_factor_to_gain_db(float factor)

float ma_gain_db_to_factor(float gain)

ctypedef void ma_data_source

ctypedef struct {
    ma_result (*onRead)(ma_data_source *pDataSource, void *pFramesOut,
                        ma_uint64 frameCount, ma_uint64 *pFramesRead)
    ma_result (*onSeek)(ma_data_source *pDataSource, ma_uint64 frameIndex)
    ma_result (*onMap)(ma_data_source *pDataSource, void **ppFramesOut,
                       ma_uint64 *pFrameCount)
    ma_result (*onUnmap)(ma_data_source *pDataSource, ma_uint64 frameCount)
    ma_result (*onGetDataFormat)(ma_data_source *pDataSource,
                                 ma_format *pFormat, ma_uint32 *pChannels,
                                 ma_uint32 *pSampleRate)
    ma_result (*onGetCursor)(ma_data_source *pDataSource, ma_uint64 *pCursor)
    ma_result (*onGetLength)(ma_data_source *pDataSource, ma_uint64 *pLength)
} ma_data_source_callbacks

ma_result ma_data_source_read_pcm_frames(ma_data_source *pDataSource,
                                                void *pFramesOut,
                                                ma_uint64 frameCount,
                                                ma_uint64 *pFramesRead,
                                                ma_bool32 loop)
ma_result ma_data_source_seek_pcm_frames(ma_data_source *pDataSource,
                                                ma_uint64 frameCount,
                                                ma_uint64 *pFramesSeeked,
                                                ma_bool32 loop)
ma_result ma_data_source_seek_to_pcm_frame(ma_data_source *pDataSource,
                                                  ma_uint64 frameIndex)
ma_result ma_data_source_map(ma_data_source *pDataSource,
                                    void **ppFramesOut, ma_uint64 *pFrameCount)
ma_result ma_data_source_unmap(ma_data_source *pDataSource,
                                      ma_uint64 frameCount)
ma_result ma_data_source_get_data_format(ma_data_source *pDataSource,
                                                ma_format *pFormat,
                                                ma_uint32 *pChannels,
                                                ma_uint32 *pSampleRate)
ma_result
ma_data_source_get_cursor_in_pcm_frames(ma_data_source *pDataSource,
                                        ma_uint64 *pCursor)
ma_result
ma_data_source_get_length_in_pcm_frames(ma_data_source *pDataSource,
                                        ma_uint64 *pLength)

ctypedef struct {
    ma_data_source_callbacks ds
    ma_format format
    ma_uint32 channels
    ma_uint64 cursor
    ma_uint64 sizeInFrames
    const void *pData
} ma_audio_buffer_ref

ma_result ma_audio_buffer_ref_init(ma_format format, ma_uint32 channels,
                                          const void *pData,
                                          ma_uint64 sizeInFrames,
                                          ma_audio_buffer_ref *pAudioBufferRef)
ma_result
ma_audio_buffer_ref_set_data(ma_audio_buffer_ref *pAudioBufferRef,
                             const void *pData, ma_uint64 sizeInFrames)
ma_uint64
ma_audio_buffer_ref_read_pcm_frames(ma_audio_buffer_ref *pAudioBufferRef,
                                    void *pFramesOut, ma_uint64 frameCount,
                                    ma_bool32 loop)
ma_result
ma_audio_buffer_ref_seek_to_pcm_frame(ma_audio_buffer_ref *pAudioBufferRef,
                                      ma_uint64 frameIndex)
ma_result ma_audio_buffer_ref_map(ma_audio_buffer_ref *pAudioBufferRef,
                                         void **ppFramesOut,
                                         ma_uint64 *pFrameCount)
ma_result ma_audio_buffer_ref_unmap(ma_audio_buffer_ref *pAudioBufferRef,
                                           ma_uint64 frameCount)
ma_result
ma_audio_buffer_ref_at_end(ma_audio_buffer_ref *pAudioBufferRef)
ma_result
ma_audio_buffer_ref_get_available_frames(ma_audio_buffer_ref *pAudioBufferRef,
                                         ma_uint64 *pAvailableFrames)

ctypedef struct {
    ma_format format
    ma_uint32 channels
    ma_uint64 sizeInFrames
    const void *pData
    ma_allocation_callbacks allocationCallbacks
} ma_audio_buffer_config

ma_audio_buffer_config ma_audio_buffer_config_init(
    ma_format format, ma_uint32 channels, ma_uint64 sizeInFrames,
    const void *pData, const ma_allocation_callbacks *pAllocationCallbacks)

ctypedef struct {
    ma_audio_buffer_ref ref
    ma_allocation_callbacks allocationCallbacks
    ma_bool32 ownsData
    ma_uint8 _pExtraData[1]
} ma_audio_buffer

ma_result ma_audio_buffer_init(const ma_audio_buffer_config *pConfig,
                                      ma_audio_buffer *pAudioBuffer)
ma_result
ma_audio_buffer_init_copy(const ma_audio_buffer_config *pConfig,
                          ma_audio_buffer *pAudioBuffer)
ma_result
ma_audio_buffer_alloc_and_init(const ma_audio_buffer_config *pConfig,
                               ma_audio_buffer **ppAudioBuffer)
void ma_audio_buffer_uninit(ma_audio_buffer *pAudioBuffer)
void ma_audio_buffer_uninit_and_free(ma_audio_buffer *pAudioBuffer)
ma_uint64 ma_audio_buffer_read_pcm_frames(ma_audio_buffer *pAudioBuffer,
                                                 void *pFramesOut,
                                                 ma_uint64 frameCount,
                                                 ma_bool32 loop)
ma_result
ma_audio_buffer_seek_to_pcm_frame(ma_audio_buffer *pAudioBuffer,
                                  ma_uint64 frameIndex)
ma_result ma_audio_buffer_map(ma_audio_buffer *pAudioBuffer,
                                     void **ppFramesOut,
                                     ma_uint64 *pFrameCount)
ma_result ma_audio_buffer_unmap(ma_audio_buffer *pAudioBuffer,
                                       ma_uint64 frameCount)
ma_result ma_audio_buffer_at_end(ma_audio_buffer *pAudioBuffer)
ma_result
ma_audio_buffer_get_available_frames(ma_audio_buffer *pAudioBuffer,
                                     ma_uint64 *pAvailableFrames)

ctypedef void ma_vfs
ctypedef ma_handle ma_vfs_file

ctypedef enum {
    ma_seek_origin_start,
    ma_seek_origin_current,
    ma_seek_origin_end
} ma_seek_origin

ctypedef struct {
    ma_uint64 sizeInBytes
} ma_file_info

ctypedef struct {
    ma_result (*onOpen)(ma_vfs *pVFS, const char *pFilePath, ma_uint32 openMode,
                        ma_vfs_file *pFile)
    ma_result (*onOpenW)(ma_vfs *pVFS, const wchar_t *pFilePath,
                         ma_uint32 openMode, ma_vfs_file *pFile)
    ma_result (*onClose)(ma_vfs *pVFS, ma_vfs_file file)
    ma_result (*onRead)(ma_vfs *pVFS, ma_vfs_file file, void *pDst,
                        size_t sizeInBytes, size_t *pBytesRead)
    ma_result (*onWrite)(ma_vfs *pVFS, ma_vfs_file file, const void *pSrc,
                         size_t sizeInBytes, size_t *pBytesWritten)
    ma_result (*onSeek)(ma_vfs *pVFS, ma_vfs_file file, ma_int64 offset,
                        ma_seek_origin origin)
    ma_result (*onTell)(ma_vfs *pVFS, ma_vfs_file file, ma_int64 *pCursor)
    ma_result (*onInfo)(ma_vfs *pVFS, ma_vfs_file file, ma_file_info *pInfo)
} ma_vfs_callbacks

ma_result ma_vfs_open(ma_vfs *pVFS, const char *pFilePath,
                             ma_uint32 openMode, ma_vfs_file *pFile)
ma_result ma_vfs_open_w(ma_vfs *pVFS, const wchar_t *pFilePath,
                               ma_uint32 openMode, ma_vfs_file *pFile)
ma_result ma_vfs_close(ma_vfs *pVFS, ma_vfs_file file)
ma_result ma_vfs_read(ma_vfs *pVFS, ma_vfs_file file, void *pDst,
                             size_t sizeInBytes, size_t *pBytesRead)
ma_result ma_vfs_write(ma_vfs *pVFS, ma_vfs_file file, const void *pSrc,
                              size_t sizeInBytes, size_t *pBytesWritten)
ma_result ma_vfs_seek(ma_vfs *pVFS, ma_vfs_file file, ma_int64 offset,
                             ma_seek_origin origin)
ma_result ma_vfs_tell(ma_vfs *pVFS, ma_vfs_file file, ma_int64 *pCursor)
ma_result ma_vfs_info(ma_vfs *pVFS, ma_vfs_file file,
                             ma_file_info *pInfo)
ma_result
ma_vfs_open_and_read_file(ma_vfs *pVFS, const char *pFilePath, void **ppData,
                          size_t *pSize,
                          const ma_allocation_callbacks *pAllocationCallbacks)

ctypedef struct {
    ma_vfs_callbacks cb
    ma_allocation_callbacks allocationCallbacks
} ma_default_vfs

ma_result
ma_default_vfs_init(ma_default_vfs *pVFS,
                    const ma_allocation_callbacks *pAllocationCallbacks)

ctypedef enum { ma_resource_format_wav } ma_resource_format

ctypedef struct ma_decoder ma_decoder

ctypedef size_t (*ma_decoder_read_proc)(ma_decoder *pDecoder, void *pBufferOut,
                                       size_t bytesToRead)
ctypedef ma_bool32 (*ma_decoder_seek_proc)(ma_decoder *pDecoder, int byteOffset,
                                          ma_seek_origin origin)
ctypedef ma_uint64 (*ma_decoder_read_pcm_frames_proc)(ma_decoder *pDecoder,
                                                     void *pFramesOut,
                                                     ma_uint64 frameCount)
ctypedef ma_result (*ma_decoder_seek_to_pcm_frame_proc)(ma_decoder *pDecoder,
                                                       ma_uint64 frameIndex)
ctypedef ma_result (*ma_decoder_uninit_proc)(ma_decoder *pDecoder)
ctypedef ma_uint64 (*ma_decoder_get_length_in_pcm_frames_proc)(
    ma_decoder *pDecoder)

ctypedef struct {
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_channel channelMap[32]
    ma_channel_mix_mode channelMixMode
    ma_dither_mode ditherMode
    struct {
        ma_resample_algorithm algorithm
        struct {
            ma_uint32 lpfOrder
        } linear
        struct {
            int quality
        } speex
    } resampling
    ma_allocation_callbacks allocationCallbacks
} ma_decoder_config

struct ma_decoder {
    ma_data_source_callbacks ds
    ma_decoder_read_proc onRead
    ma_decoder_seek_proc onSeek
    void *pUserData
    ma_uint64 readPointerInBytes
    ma_uint64 readPointerInPCMFrames
    ma_format internalFormat
    ma_uint32 internalChannels
    ma_uint32 internalSampleRate
    ma_channel internalChannelMap[32]
    ma_format outputFormat
    ma_uint32 outputChannels
    ma_uint32 outputSampleRate
    ma_channel outputChannelMap[32]
    ma_data_converter converter
    ma_allocation_callbacks allocationCallbacks
    ma_decoder_read_pcm_frames_proc onReadPCMFrames
    ma_decoder_seek_to_pcm_frame_proc onSeekToPCMFrame
    ma_decoder_uninit_proc onUninit
    ma_decoder_get_length_in_pcm_frames_proc onGetLengthInPCMFrames
    void *pInternalDecoder
    union {
        struct {
            ma_vfs *pVFS
            ma_vfs_file file
        } vfs
        struct {
            const ma_uint8 *pData
            size_t dataSize
            size_t currentReadPos
        } memory
    } backend
}

ma_decoder_config ma_decoder_config_init(ma_format outputFormat,
                                                ma_uint32 outputChannels,
                                                ma_uint32 outputSampleRate)

ma_result ma_decoder_init(ma_decoder_read_proc onRead,
                                 ma_decoder_seek_proc onSeek, void *pUserData,
                                 const ma_decoder_config *pConfig,
                                 ma_decoder *pDecoder)
ma_result ma_decoder_init_wav(ma_decoder_read_proc onRead,
                                     ma_decoder_seek_proc onSeek,
                                     void *pUserData,
                                     const ma_decoder_config *pConfig,
                                     ma_decoder *pDecoder)
ma_result ma_decoder_init_flac(ma_decoder_read_proc onRead,
                                      ma_decoder_seek_proc onSeek,
                                      void *pUserData,
                                      const ma_decoder_config *pConfig,
                                      ma_decoder *pDecoder)
ma_result ma_decoder_init_mp3(ma_decoder_read_proc onRead,
                                     ma_decoder_seek_proc onSeek,
                                     void *pUserData,
                                     const ma_decoder_config *pConfig,
                                     ma_decoder *pDecoder)
ma_result ma_decoder_init_vorbis(ma_decoder_read_proc onRead,
                                        ma_decoder_seek_proc onSeek,
                                        void *pUserData,
                                        const ma_decoder_config *pConfig,
                                        ma_decoder *pDecoder)
ma_result
ma_decoder_init_raw(ma_decoder_read_proc onRead, ma_decoder_seek_proc onSeek,
                    void *pUserData, const ma_decoder_config *pConfigIn,
                    const ma_decoder_config *pConfigOut, ma_decoder *pDecoder)

ma_result ma_decoder_init_memory(const void *pData, size_t dataSize,
                                        const ma_decoder_config *pConfig,
                                        ma_decoder *pDecoder)
ma_result ma_decoder_init_memory_wav(const void *pData, size_t dataSize,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder)
ma_result ma_decoder_init_memory_flac(const void *pData, size_t dataSize,
                                             const ma_decoder_config *pConfig,
                                             ma_decoder *pDecoder)
ma_result ma_decoder_init_memory_mp3(const void *pData, size_t dataSize,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder)
ma_result ma_decoder_init_memory_vorbis(const void *pData,
                                               size_t dataSize,
                                               const ma_decoder_config *pConfig,
                                               ma_decoder *pDecoder)
ma_result ma_decoder_init_memory_raw(const void *pData, size_t dataSize,
                                            const ma_decoder_config *pConfigIn,
                                            const ma_decoder_config *pConfigOut,
                                            ma_decoder *pDecoder)

ma_result ma_decoder_init_vfs(ma_vfs *pVFS, const char *pFilePath,
                                     const ma_decoder_config *pConfig,
                                     ma_decoder *pDecoder)
ma_result ma_decoder_init_vfs_wav(ma_vfs *pVFS, const char *pFilePath,
                                         const ma_decoder_config *pConfig,
                                         ma_decoder *pDecoder)
ma_result ma_decoder_init_vfs_flac(ma_vfs *pVFS, const char *pFilePath,
                                          const ma_decoder_config *pConfig,
                                          ma_decoder *pDecoder)
ma_result ma_decoder_init_vfs_mp3(ma_vfs *pVFS, const char *pFilePath,
                                         const ma_decoder_config *pConfig,
                                         ma_decoder *pDecoder)
ma_result ma_decoder_init_vfs_vorbis(ma_vfs *pVFS, const char *pFilePath,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder)

ma_result ma_decoder_init_vfs_w(ma_vfs *pVFS, const wchar_t *pFilePath,
                                       const ma_decoder_config *pConfig,
                                       ma_decoder *pDecoder)
ma_result ma_decoder_init_vfs_wav_w(ma_vfs *pVFS,
                                           const wchar_t *pFilePath,
                                           const ma_decoder_config *pConfig,
                                           ma_decoder *pDecoder)
ma_result ma_decoder_init_vfs_flac_w(ma_vfs *pVFS,
                                            const wchar_t *pFilePath,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder)
ma_result ma_decoder_init_vfs_mp3_w(ma_vfs *pVFS,
                                           const wchar_t *pFilePath,
                                           const ma_decoder_config *pConfig,
                                           ma_decoder *pDecoder)
ma_result ma_decoder_init_vfs_vorbis_w(ma_vfs *pVFS,
                                              const wchar_t *pFilePath,
                                              const ma_decoder_config *pConfig,
                                              ma_decoder *pDecoder)

ma_result ma_decoder_init_file(const char *pFilePath,
                                      const ma_decoder_config *pConfig,
                                      ma_decoder *pDecoder)
ma_result ma_decoder_init_file_wav(const char *pFilePath,
                                          const ma_decoder_config *pConfig,
                                          ma_decoder *pDecoder)
ma_result ma_decoder_init_file_flac(const char *pFilePath,
                                           const ma_decoder_config *pConfig,
                                           ma_decoder *pDecoder)
ma_result ma_decoder_init_file_mp3(const char *pFilePath,
                                          const ma_decoder_config *pConfig,
                                          ma_decoder *pDecoder)
ma_result ma_decoder_init_file_vorbis(const char *pFilePath,
                                             const ma_decoder_config *pConfig,
                                             ma_decoder *pDecoder)

ma_result ma_decoder_init_file_w(const wchar_t *pFilePath,
                                        const ma_decoder_config *pConfig,
                                        ma_decoder *pDecoder)
ma_result ma_decoder_init_file_wav_w(const wchar_t *pFilePath,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder)
ma_result ma_decoder_init_file_flac_w(const wchar_t *pFilePath,
                                             const ma_decoder_config *pConfig,
                                             ma_decoder *pDecoder)
ma_result ma_decoder_init_file_mp3_w(const wchar_t *pFilePath,
                                            const ma_decoder_config *pConfig,
                                            ma_decoder *pDecoder)
ma_result ma_decoder_init_file_vorbis_w(const wchar_t *pFilePath,
                                               const ma_decoder_config *pConfig,
                                               ma_decoder *pDecoder)

ma_result ma_decoder_uninit(ma_decoder *pDecoder)

ma_result ma_decoder_get_cursor_in_pcm_frames(ma_decoder *pDecoder,
                                                     ma_uint64 *pCursor)

ma_uint64 ma_decoder_get_length_in_pcm_frames(ma_decoder *pDecoder)

ma_uint64 ma_decoder_read_pcm_frames(ma_decoder *pDecoder,
                                            void *pFramesOut,
                                            ma_uint64 frameCount)

ma_result ma_decoder_seek_to_pcm_frame(ma_decoder *pDecoder,
                                              ma_uint64 frameIndex)

ma_result ma_decoder_get_available_frames(ma_decoder *pDecoder,
                                                 ma_uint64 *pAvailableFrames)

ma_result ma_decode_from_vfs(ma_vfs *pVFS, const char *pFilePath,
                                    ma_decoder_config *pConfig,
                                    ma_uint64 *pFrameCountOut,
                                    void **ppPCMFramesOut)
ma_result ma_decode_file(const char *pFilePath,
                                ma_decoder_config *pConfig,
                                ma_uint64 *pFrameCountOut,
                                void **ppPCMFramesOut)
ma_result ma_decode_memory(const void *pData, size_t dataSize,
                                  ma_decoder_config *pConfig,
                                  ma_uint64 *pFrameCountOut,
                                  void **ppPCMFramesOut)

ctypedef struct ma_encoder ma_encoder

ctypedef size_t (*ma_encoder_write_proc)(ma_encoder *pEncoder,
                                        const void *pBufferIn,
                                        size_t bytesToWrite)
ctypedef ma_bool32 (*ma_encoder_seek_proc)(ma_encoder *pEncoder, int byteOffset,
                                          ma_seek_origin origin)
ctypedef ma_result (*ma_encoder_init_proc)(ma_encoder *pEncoder)
ctypedef void (*ma_encoder_uninit_proc)(ma_encoder *pEncoder)
ctypedef ma_uint64 (*ma_encoder_write_pcm_frames_proc)(ma_encoder *pEncoder,
                                                      const void *pFramesIn,
                                                      ma_uint64 frameCount)

ctypedef struct {
    ma_resource_format resourceFormat
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_allocation_callbacks allocationCallbacks
} ma_encoder_config

ma_encoder_config
ma_encoder_config_init(ma_resource_format resourceFormat, ma_format format,
                       ma_uint32 channels, ma_uint32 sampleRate)

struct ma_encoder {
    ma_encoder_config config
    ma_encoder_write_proc onWrite
    ma_encoder_seek_proc onSeek
    ma_encoder_init_proc onInit
    ma_encoder_uninit_proc onUninit
    ma_encoder_write_pcm_frames_proc onWritePCMFrames
    void *pUserData
    void *pInternalEncoder
    void *pFile
}

ma_result ma_encoder_init(ma_encoder_write_proc onWrite,
                                 ma_encoder_seek_proc onSeek, void *pUserData,
                                 const ma_encoder_config *pConfig,
                                 ma_encoder *pEncoder)
ma_result ma_encoder_init_file(const char *pFilePath,
                                      const ma_encoder_config *pConfig,
                                      ma_encoder *pEncoder)
ma_result ma_encoder_init_file_w(const wchar_t *pFilePath,
                                        const ma_encoder_config *pConfig,
                                        ma_encoder *pEncoder)
void ma_encoder_uninit(ma_encoder *pEncoder)
ma_uint64 ma_encoder_write_pcm_frames(ma_encoder *pEncoder,
                                             const void *pFramesIn,
                                             ma_uint64 frameCount)

ctypedef enum {
    ma_waveform_type_sine,
    ma_waveform_type_square,
    ma_waveform_type_triangle,
    ma_waveform_type_sawtooth
} ma_waveform_type

ctypedef struct {
    ma_format format
    ma_uint32 channels
    ma_uint32 sampleRate
    ma_waveform_type type
    double amplitude
    double frequency
} ma_waveform_config

ma_waveform_config
ma_waveform_config_init(ma_format format, ma_uint32 channels,
                        ma_uint32 sampleRate, ma_waveform_type type,
                        double amplitude, double frequency)

ctypedef struct {
    ma_data_source_callbacks ds
    ma_waveform_config config
    double advance
    double time
} ma_waveform

ma_result ma_waveform_init(const ma_waveform_config *pConfig,
                                  ma_waveform *pWaveform)
ma_uint64 ma_waveform_read_pcm_frames(ma_waveform *pWaveform,
                                             void *pFramesOut,
                                             ma_uint64 frameCount)
ma_result ma_waveform_seek_to_pcm_frame(ma_waveform *pWaveform,
                                               ma_uint64 frameIndex)
ma_result ma_waveform_set_amplitude(ma_waveform *pWaveform,
                                           double amplitude)
ma_result ma_waveform_set_frequency(ma_waveform *pWaveform,
                                           double frequency)
ma_result ma_waveform_set_type(ma_waveform *pWaveform,
                                      ma_waveform_type type)
ma_result ma_waveform_set_sample_rate(ma_waveform *pWaveform,
                                             ma_uint32 sampleRate)

ctypedef enum {
    ma_noise_type_white,
    ma_noise_type_pink,
    ma_noise_type_brownian
} ma_noise_type

ctypedef struct {
    ma_format format
    ma_uint32 channels
    ma_noise_type type
    ma_int32 seed
    double amplitude
    ma_bool32 duplicateChannels
} ma_noise_config

ma_noise_config ma_noise_config_init(ma_format format,
                                            ma_uint32 channels,
                                            ma_noise_type type, ma_int32 seed,
                                            double amplitude)

ctypedef struct {
    ma_data_source_callbacks ds
    ma_noise_config config
    ma_lcg lcg
    union {
        struct {
            double bin[32][16]
            double accumulation[32]
            ma_uint32 counter[32]
        } pink
        struct {
            double accumulation[32]
        } brownian
    } state
} ma_noise

ma_result ma_noise_init(const ma_noise_config *pConfig,
                               ma_noise *pNoise)
ma_uint64 ma_noise_read_pcm_frames(ma_noise *pNoise, void *pFramesOut,
                                          ma_uint64 frameCount)
ma_result ma_noise_set_amplitude(ma_noise *pNoise, double amplitude)
ma_result ma_noise_set_seed(ma_noise *pNoise, ma_int32 seed)
ma_result ma_noise_set_type(ma_noise *pNoise, ma_noise_type type)

