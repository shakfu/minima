

all: minima

miniaudio/libminiaudio.a:
	@gcc -c miniaudio/miniaudio.c -o miniaudio/miniaudio.o
	@ar -rc miniaudio/libminiaudio.a miniaudio/miniaudio.o
	@rm -f miniaudio/miniaudio.o

minima: miniaudio/libminiaudio.a
	@python3 setup.py build_ext --inplace	
	@rm -rf ./build ./minima.c	

.PHONY: test clean reset

test:
	python3 test_minima.py

clean:
	@rm -f *.so

reset: clean
	@rm -f miniaudio/libminiaudio.a

