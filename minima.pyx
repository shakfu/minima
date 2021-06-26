cimport libminiaudio


DEF MA_SUCCESS = 0

DEF MA_NO_DECODING = 1
DEF MA_NO_ENCODING = 1

DEF DEVICE_CHANNELS    = 2
DEF DEVICE_SAMPLE_RATE = 48000


def get_version():
    print(libminiaudio.ma_version_string())

cdef void data_callback(libminiaudio.ma_device* device, 
                        void* output, const void* input_, libminiaudio.ma_uint32 frame_count) nogil:

    cdef libminiaudio.ma_waveform* sinewave

    #assert device.playback.channels == DEVICE_CHANNELS

    sinewave = <libminiaudio.ma_waveform*>device.pUserData
    #MA_ASSERT(sinewave != NULL)

    libminiaudio.ma_waveform_read_pcm_frames(sinewave, output, frame_count)


def main():
    cdef libminiaudio.ma_waveform sineWave
    cdef libminiaudio.ma_device_config deviceConfig
    cdef libminiaudio.ma_device device
    cdef libminiaudio.ma_waveform_config sineWaveConfig

    sineWaveConfig = libminiaudio.ma_waveform_config_init(
        libminiaudio.ma_format_f32, 
        DEVICE_CHANNELS, DEVICE_SAMPLE_RATE, 
        libminiaudio.ma_waveform_type_sine, 0.2, 220)

    libminiaudio.ma_waveform_init(&sineWaveConfig, &sineWave)

    deviceConfig = libminiaudio.ma_device_config_init(libminiaudio.ma_device_type_playback)
    deviceConfig.playback.format   = libminiaudio.ma_format_f32
    deviceConfig.playback.channels = DEVICE_CHANNELS
    deviceConfig.sampleRate        = DEVICE_SAMPLE_RATE
    deviceConfig.dataCallback      = data_callback
    deviceConfig.pUserData         = &sineWave

    if (libminiaudio.ma_device_init(NULL, &deviceConfig, &device) != MA_SUCCESS):
        print("Failed to open playback device.")
        return -4

    print("Device Name: %s" % device.playback.name)

    if (libminiaudio.ma_device_start(&device) != MA_SUCCESS):
        print("Failed to start playback device.\n")
        libminiaudio.ma_device_uninit(&device)
        return -5
    
    if input("Press Enter to quit...\n") == '':
        libminiaudio.ma_device_uninit(&device)
