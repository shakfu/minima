import minima


def test_playback():
	minima.play_file('resources/beat.wav')

def test_play_sine():
	minima.play_sine()
	
if __name__ == '__main__':
	test_play_sine()
	test_playback()
