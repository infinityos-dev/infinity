MAKEFLAGS += -rR
.SUFFIXES:

override USER_VARIABLE = $(if $(filter $(origin $(1)),default undefined),$(eval override $(1) := $(2)))

$(call USER_VARIABLE,KARCH,x86_64)
$(call USER_VARIABLE,QEMUFLAGS,-m 2G -d int -D log/interrupts.txt -M smm=off -no-reboot -no-shutdown -serial stdio -s)

override IMAGE_NAME := hexium_os-$(KARCH)

.PHONY: all
all: $(IMAGE_NAME).iso

.PHONY: setup
setup:
	mkdir -p log/
	touch log/interrupts.txt

.PHONY: run
run: run-$(KARCH)

.PHONY: run-x86_64
run-x86_64: ovmf/ovmf-code-$(KARCH).fd ovmf/ovmf-vars-$(KARCH).fd $(IMAGE_NAME).iso setup
	qemu-system-$(KARCH) \
		-M q35 \
		-drive if=pflash,unit=0,format=raw,file=ovmf/ovmf-code-$(KARCH).fd,readonly=on \
		-drive if=pflash,unit=1,format=raw,file=ovmf/ovmf-vars-$(KARCH).fd \
		-cdrom $(IMAGE_NAME).iso \
		$(QEMUFLAGS)

.PHONY: test
test: test-iso
	@set -e; \
	FAILED=0; \
	echo "\n\n\n--------RUNNING KERNEL INTEGRATION TESTS-------\n\n"; \
	for iso in hexium_os-tests/*.iso; do \
		echo "==============================="; \
		echo "Running integration test: $$iso"; \
		echo "-------------------------------"; \
		if qemu-system-x86_64 \
			-cdrom "$$iso" \
			-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
			-serial stdio -display none; \
		then \
			echo "✅ Integration Test passed: $$iso"; \
		elif [ $$? -eq 33 ]; then \
			echo "✅ Integration Test passed (exit 33): $$iso"; \
		else \
			echo "❌ Integration Test failed: $$iso"; \
			FAILED=1; \
		fi; \
		echo ""; \
	done; \
	exit $$FAILED

ovmf/ovmf-code-$(KARCH).fd:
	mkdir -p ovmf
	curl -Lo $@ https://github.com/osdev0/edk2-ovmf-nightly/releases/latest/download/ovmf-code-$(KARCH).fd

ovmf/ovmf-vars-$(KARCH).fd:
	mkdir -p ovmf
	curl -Lo $@ https://github.com/osdev0/edk2-ovmf-nightly/releases/latest/download/ovmf-vars-$(KARCH).fd

limine/limine:
	rm -rf limine
	git clone https://github.com/limine-bootloader/limine.git --branch=v9.x-binary --depth=1
	$(MAKE) -C limine

.PHONY: kernel
kernel:
	$(MAKE) -C kernel

.PHONY: kernel-test
kernel-test:
	$(MAKE) -C kernel test

.PHONY: ramfs
ramfs:
	mkdir -p initrd/
	./tools/gen-initrd.sh initrd ustar

$(IMAGE_NAME).iso: limine/limine kernel ramfs
	rm -rf iso_root
	mkdir -p iso_root/boot
	cp -v kernel/kernel iso_root/boot/
	cp -v ramfs.img iso_root/boot/
	mkdir -p iso_root/boot/limine
	cp -v limine.conf iso_root/boot/limine/
	mkdir -p iso_root/EFI/BOOT
	cp -v limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/boot/limine/
	cp -v limine/BOOTX64.EFI iso_root/EFI/BOOT/
	cp -v limine/BOOTIA32.EFI iso_root/EFI/BOOT/
	xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		iso_root -o $(IMAGE_NAME).iso
	./limine/limine bios-install $(IMAGE_NAME).iso
	rm -rf iso_root

.PHONY: test-iso
test-iso: limine/limine ramfs kernel-test
	mkdir -p hexium_os-tests
	for testbin in kernel/kernel-test/*; do \
		testname=$$(basename $$testbin); \
		isodir=iso_root_$$testname; \
		mkdir -p $$isodir/boot/limine $$isodir/EFI/BOOT; \
		cp -v $$testbin $$isodir/boot/kernel; \
		cp -v ramfs.img $$isodir/boot/; \
		cp -v limine.conf $$isodir/boot/limine/; \
		cp -v limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin $$isodir/boot/limine/; \
		cp -v limine/BOOTX64.EFI $$isodir/EFI/BOOT/; \
		cp -v limine/BOOTIA32.EFI $$isodir/EFI/BOOT/; \
		xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
			-no-emul-boot -boot-load-size 4 -boot-info-table \
			--efi-boot boot/limine/limine-uefi-cd.bin \
			-efi-boot-part --efi-boot-image --protective-msdos-label \
			$$isodir -o hexium_os-tests/hexium_os-$$testname.iso; \
		./limine/limine bios-install hexium_os-tests/hexium_os-$$testname.iso; \
		rm -rf $$isodir; \
	done

.PHONY: clean
clean:
	$(MAKE) -C kernel clean
	rm -rf iso_root *.iso
	rm -rf hexium_os-tests

.PHONY: distclean
distclean: clean
	$(MAKE) -C kernel distclean
	rm -rf limine ovmf
