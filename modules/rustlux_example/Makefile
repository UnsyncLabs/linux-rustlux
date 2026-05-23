# SPDX-License-Identifier: GPL-2.0-only
# Makefile para compilar el módulo fuera del árbol del kernel
#
# Uso:
#   make                    — compilar contra el kernel actual
#   make KDIR=/path/to/src  — compilar contra un kernel específico
#   make clean              — limpiar artefactos

KDIR ?= /lib/modules/$(shell uname -r)/build
PWD  := $(shell pwd)

default:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean

install:
	$(MAKE) -C $(KDIR) M=$(PWD) modules_install

.PHONY: default clean install
