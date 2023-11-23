
.PHONY: inplace test clean reset

all: minima

miniaudio/libminiaudio.a:
	@gcc -c miniaudio/miniaudio.c -o miniaudio/miniaudio.o
	@ar -rc miniaudio/libminiaudio.a miniaudio/miniaudio.o
	@rm -f miniaudio/miniaudio.o

minima: miniaudio/libminiaudio.a
	@python3 setup.py build --build-lib=build
	@rm -rf ./minima.c

inplace: miniaudio/libminiaudio.a
	@python3 setup.py build_ext --inplace	
	@rm -rf ./build ./minima.c	

test:
	PYTHONPATH=`pwd`/build python3 tests/test_minima.py

clean:
	@rm -f *.so

reset: clean
	@rm -f miniaudio/libminiaudio.a

