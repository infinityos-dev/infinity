MAKEFLAGS += -rR
.SUFFIXES:

override OUTPUT := kernel

override USER_VARIABLE = $(if $(filter $(origin $(1)),default undefined),$(eval override $(1) := $(2)))

$(call USER_VARIABLE,KARCH,x86_64)

ifeq ($(RUST_TARGET),)
    override RUST_TARGET := $(KARCH)-unknown-none
	ifeq ($(KARCH),riscv64)
    	override RUST_TARGET := riscv64gc-unknown-none-elf
	endif
endif

ifeq ($(RUST_PROFILE),)
    override RUST_PROFILE := dev
endif

override RUST_PROFILE_SUBDIR := $(RUST_PROFILE)
ifeq ($(RUST_PROFILE),dev)
    override RUST_PROFILE_SUBDIR := debug
endif

.PHONY: all
all:
	RUSTFLAGS="-C relocation-model=static -C force-frame-pointers=yes" cargo build --target $(RUST_TARGET) --profile $(RUST_PROFILE)
	cp target/$(RUST_TARGET)/$(RUST_PROFILE_SUBDIR)/hexium_os kernel

test:
	mkdir -p kernel-test
	RUSTFLAGS="-C relocation-model=static" cargo test --no-run --features testing --target $(RUST_TARGET) --profile $(RUST_PROFILE) --color always \
		2>&1 | tee .test-log.txt
	@grep -o 'target/[^ )]*' .test-log.txt | while read -r path; do \
		if [ -x "$$path" ]; then \
			echo "Copying $$path to kernel-test/"; \
			cp "$$path" kernel-test/; \
		else \
			echo "Skipping non-executable: $$path"; \
		fi \
	done

	@rm .test-log.txt

.PHONY: clean
clean:
	cargo clean
	rm -rf kernel
	rm -rf kernel-test

.PHONY: distclean
distclean: clean
