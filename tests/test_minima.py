import minima

SOUNDFILE='tests/beat.wav'

def test_play_sine():
	print("testing playing a generated sine...")
	minima.play_sine()
	
def test_play_filek():
	print("testing playing a file using callback api...")
	minima.play_file(SOUNDFILE)

def test_list_devices():
	print("testing listing available devices...")

	minima.list_devices()

def test_engine_play_file():
	print("testing playing a file using engine api...")
	minima.engine_play_file(SOUNDFILE)

if __name__ == '__main__':
	version = minima.get_version()
	print(f"minima 0.0.1: miniaudio {version}")
	test_list_devices()
	test_play_sine()
	test_play_filek()
	test_engine_play_file()
