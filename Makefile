

all: minima

minima:
	@python3 setup.py build_ext --inplace	
	@rm -rf ./build ./minima.c	

.PHONY: test clean

test:
	@python3 ./tests/test_minima.py

clean:
	@rm -f *.so
