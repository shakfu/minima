cimport libminiaudio as lib
from enum import Enum

DEF MA_SUCCESS = 0

DEF MA_NO_DECODING = 1
DEF MA_NO_ENCODING = 1

DEF DEVICE_CHANNELS    = 2
DEF DEVICE_SAMPLE_RATE = 48000


def get_version():
    print(lib.ma_version_string())


cdef void sine_data_callback(lib.ma_device* device, 
                            void* output, 
                            const void* input_, 
                            lib.ma_uint32 frame_count) noexcept nogil:

    cdef lib.ma_waveform* sinewave

    #assert device.playback.channels == DEVICE_CHANNELS

    sinewave = <lib.ma_waveform*>device.pUserData
    #MA_ASSERT(sinewave != NULL)

    lib.ma_waveform_read_pcm_frames(sinewave, output, frame_count, NULL)


def play_sine(double amp=0.2, double freq=220):
    cdef lib.ma_waveform sineWave
    cdef lib.ma_device_config deviceConfig
    cdef lib.ma_device device
    cdef lib.ma_waveform_config sineWaveConfig

    sineWaveConfig = lib.ma_waveform_config_init(
        lib.ma_format_f32, 
        DEVICE_CHANNELS, DEVICE_SAMPLE_RATE,
        lib.ma_waveform_type_sine, amp, freq)

    lib.ma_waveform_init(&sineWaveConfig, &sineWave)

    deviceConfig = lib.ma_device_config_init(lib.ma_device_type_playback)
    deviceConfig.playback.format   = lib.ma_format_f32
    deviceConfig.playback.channels = DEVICE_CHANNELS
    deviceConfig.sampleRate        = DEVICE_SAMPLE_RATE
    deviceConfig.dataCallback      = sine_data_callback
    deviceConfig.pUserData         = &sineWave

    if (lib.ma_device_init(NULL, &deviceConfig, &device) != MA_SUCCESS):
        print("Failed to open playback device.")
        return -4

    print("Device Name: %s" % device.playback.name)

    if (lib.ma_device_start(&device) != MA_SUCCESS):
        print("Failed to start playback device.\n")
        lib.ma_device_uninit(&device)
        return -5
    
    if input("Press Enter to quit...\n") == '':
        lib.ma_device_uninit(&device)






cdef void file_data_callback(lib.ma_device* device, 
                            void* output, const void* input_,
                            lib.ma_uint32 frame_count) noexcept nogil:
    
    cdef lib.ma_decoder* decoder = <lib.ma_decoder*>device.pUserData
    
    if (decoder == NULL):
        return

    lib.ma_decoder_read_pcm_frames(decoder, output, frame_count, NULL)


def play_file(str path):

    cdef lib.ma_result result
    cdef lib.ma_decoder decoder
    cdef lib.ma_device_config deviceConfig
    cdef lib.ma_device device

    result = lib.ma_decoder_init_file(path.encode('utf8'), NULL, &decoder)
    if (result != MA_SUCCESS):
        return

    deviceConfig = lib.ma_device_config_init(lib.ma_device_type_playback)
    deviceConfig.playback.format   = decoder.outputFormat
    deviceConfig.playback.channels = decoder.outputChannels
    deviceConfig.sampleRate        = decoder.outputSampleRate
    deviceConfig.dataCallback      = file_data_callback
    deviceConfig.pUserData         = &decoder

    if (lib.ma_device_init(NULL, &deviceConfig, &device) != MA_SUCCESS):
        print("Failed to open playback device.")
        lib.ma_decoder_uninit(&decoder)
        return


    if (lib.ma_device_start(&device) != MA_SUCCESS):
        print("Failed to start playback device.")
        lib.ma_device_uninit(&device)
        lib.ma_decoder_uninit(&decoder)
        return

    if input("Press Enter to quit...\n") == '':
        lib.ma_device_uninit(&device)
        lib.ma_decoder_uninit(&decoder)



