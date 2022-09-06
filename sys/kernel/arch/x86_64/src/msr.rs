/*
 * Copyright (c) 2022 xvanc <xvancm@gmail.com>
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice,
 *    this list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * 3. Neither the name of the copyright holder nor the names of its contributors
 *    may be used to endorse or promote products derived from this software without
 *    specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY
 * EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 * SPDX-License-Identifier: BSD-3-Clause
 */

pub unsafe fn wrmsr(addr: u32, value: u64) {
    asm!(
        "wrmsr",
        in("ecx") addr,
        in("edx") value >> 32,
        in("eax") value,
        options(nomem, nostack, preserves_flags)
    );
}

pub unsafe fn rdmsr(addr: u32) -> u64 {
    let value_lo: u32;
    let value_hi: u32;

    asm!("rdmsr", in("ecx") addr, out("edx") value_hi, out("eax") value_lo, options(nomem, nostack, preserves_flags));

    (value_hi as u64) << 32 | value_lo as u64
}

pub use msr_consts::*;
#[rustfmt::skip]
mod msr_consts {
    #![allow(dead_code)]

    macro_rules! define_msrs {
        ($($(#[$m:meta])* const $name:ident = $addr:literal;)*) => {
            $($(#[$m])* pub const $name: u32 = $addr;)*
        };
    }

    define_msrs! {
        /*
         * Architectural
         */
        const IA32_P5_MC_ADDR                       = 0x00000000;
        const IA32_P5_MC_TYPE                       = 0x00000001;
        const IA32_MONITOR_FILTER_SIZE              = 0x00000006;
        const IA32_TIME_STAMP_COUNTER               = 0x00000010;
        const IA32_PLATFORM_ID                      = 0x00000017;
        const IA32_APIC_BASE                        = 0x0000001B;
        const IA32_FEATURE_CONTROL                  = 0x0000003A;
        const IA32_TSC_ADJUST                       = 0x0000003B;
        const IA32_SPEC_CTRL                        = 0x00000048;
        const IA32_PRED_CMD                         = 0x00000049;
        const IA32_PPIN_CTL                         = 0x0000004E;
        const IA32_PPIN                             = 0x0000004F;
        const IA32_BIOS_UPDT_TRIG                   = 0x00000079;
        const IA32_BIOS_SIGN_ID                     = 0x0000008B;
        const IA32_SGXLEPUBKEYHASH0                 = 0x0000008C;
        const IA32_SGXLEPUBKEYHASH1                 = 0x0000008D;
        const IA32_SGXLEPUBKEYHASH2                 = 0x0000008E;
        const IA32_SGXLEPUBKEYHASH3                 = 0x0000008F;
        const IA32_SMM_MONITOR_CTL                  = 0x0000009B;
        const IA32_SMBASE                           = 0x0000009E;
        const IA32_MISC_PACKAGE_CTLS                = 0x000000BC;
        const IA32_PMC0                             = 0x000000C1;
        const IA32_PMC1                             = 0x000000C2;
        const IA32_PMC2                             = 0x000000C3;
        const IA32_PMC3                             = 0x000000C4;
        const IA32_PMC4                             = 0x000000C5;
        const IA32_PMC5                             = 0x000000C6;
        const IA32_PMC6                             = 0x000000C7;
        const IA32_PMC7                             = 0x000000C8;
        const IA32_CORE_CAPABILITIES                = 0x000000CF;
        const IA32_UMWAIT_CONTROL                   = 0x000000E1;
        const IA32_MPERF                            = 0x000000E7;
        const IA32_APERF                            = 0x000000E8;
        const IA32_MTRRCAP                          = 0x000000FE;
        const IA32_ARCH_CAPABILITIES                = 0x0000010A;
        const IA32_FLUSH_CMD                        = 0x0000010B;
        const IA32_TSX_CTRL                         = 0x00000122;
        const IA32_SYSENTER_CS                      = 0x00000174;
        const IA32_SYSENTER_ESP                     = 0x00000175;
        const IA32_SYSENTER_EIP                     = 0x00000176;
        const IA32_MCG_CAP                          = 0x00000179;
        const IA32_MCG_STATUS                       = 0x0000017A;
        const IA32_MCG_CTL                          = 0x0000017B;
        const IA32_PERFEVTSEL0                      = 0x00000186;
        const IA32_PERFEVTSEL1                      = 0x00000187;
        const IA32_PERFEVTSEL2                      = 0x00000188;
        const IA32_PERFEVTSEL3                      = 0x00000189;
        const IA32_PERFEVTSEL4                      = 0x0000018A;
        const IA32_PERFEVTSEL5                      = 0x0000018B;
        const IA32_PERFEVTSEL6                      = 0x0000018C;
        const IA32_PERFEVTSEL7                      = 0x0000018D;
        const IA32_PERF_STATUS                      = 0x00000198;
        const IA32_PERF_CTL                         = 0x00000199;
        const IA32_CLOCK_MODULATION                 = 0x0000019A;
        const IA32_THERM_INTERRUPT                  = 0x0000019B;
        const IA32_THERM_STATUS                     = 0x0000019C;
        const IA32_MISC_ENABLE                      = 0x000001A0;
        const IA32_ENERGY_PERF_BIAS                 = 0x000001B0;
        const IA32_PACKAGE_THERM_STATUS             = 0x000001B1;
        const IA32_PACKAGE_THERM_INTERRUPT          = 0x000001B2;
        const IA32_DEBUGCTL                         = 0x000001D9;
        const IA32_LER_FROM_IP                      = 0x000001DD;
        const IA32_LER_TO_IP                        = 0x000001DE;
        const IA32_LER_INFO                         = 0x000001E0;
        const IA32_SMRR_PHYSBASE                    = 0x000001F2;
        const IA32_SMRR_PHYSMASK                    = 0x000001F3;
        const IA32_PLATFORM_DCA_CAP                 = 0x000001F8;
        const IA32_CPU_DCA_CAP                      = 0x000001F9;
        const IA32_DCA_0_CAP                        = 0x000001FA;
        const IA32_MTRR_PHYSBASE0                   = 0x00000200;
        const IA32_MTRR_PHYSMASK0                   = 0x00000201;
        const IA32_MTRR_PHYSBASE1                   = 0x00000202;
        const IA32_MTRR_PHYSMASK1                   = 0x00000203;
        const IA32_MTRR_PHYSBASE2                   = 0x00000204;
        const IA32_MTRR_PHYSMASK2                   = 0x00000205;
        const IA32_MTRR_PHYSBASE3                   = 0x00000206;
        const IA32_MTRR_PHYSMASK3                   = 0x00000207;
        const IA32_MTRR_PHYSBASE4                   = 0x00000208;
        const IA32_MTRR_PHYSMASK4                   = 0x00000209;
        const IA32_MTRR_PHYSBASE5                   = 0x0000020A;
        const IA32_MTRR_PHYSMASK5                   = 0x0000020B;
        const IA32_MTRR_PHYSBASE6                   = 0x0000020C;
        const IA32_MTRR_PHYSMASK6                   = 0x0000020D;
        const IA32_MTRR_PHYSBASE7                   = 0x0000020E;
        const IA32_MTRR_PHYSMASK7                   = 0x0000020F;
        const IA32_MTRR_PHYSBASE8                   = 0x00000210;
        const IA32_MTRR_PHYSMASK8                   = 0x00000211;
        const IA32_MTRR_PHYSBASE9                   = 0x00000212;
        const IA32_MTRR_PHYSMASK9                   = 0x00000213;
        const IA32_MTRR_FIX64K_00000                = 0x00000250;
        const IA32_MTRR_FIX64K_80000                = 0x00000258;
        const IA32_MTRR_FIX64K_A0000                = 0x00000259;
        const IA32_MTRR_FIX4K_C0000                 = 0x00000268;
        const IA32_MTRR_FIX4K_C8000                 = 0x00000269;
        const IA32_MTRR_FIX4K_D0000                 = 0x0000026A;
        const IA32_MTRR_FIX4K_D8000                 = 0x0000026B;
        const IA32_MTRR_FIX4K_E0000                 = 0x0000026C;
        const IA32_MTRR_FIX4K_E8000                 = 0x0000026D;
        const IA32_MTRR_FIX4K_F0000                 = 0x0000026E;
        const IA32_MTRR_FIX4K_F8000                 = 0x0000026F;
        const IA32_PAT                              = 0x00000277;
        const IA32_MC0_CTL2                         = 0x00000280;
        const IA32_MC1_CTL2                         = 0x00000281;
        const IA32_MC2_CTL2                         = 0x00000282;
        const IA32_MC3_CTL2                         = 0x00000283;
        const IA32_MC4_CTL2                         = 0x00000284;
        const IA32_MC5_CTL2                         = 0x00000285;
        const IA32_MC6_CTL2                         = 0x00000286;
        const IA32_MC7_CTL2                         = 0x00000287;
        const IA32_MC8_CTL2                         = 0x00000288;
        const IA32_MC9_CTL2                         = 0x00000289;
        const IA32_MC10_CTL2                        = 0x0000028A;
        const IA32_MC11_CTL2                        = 0x0000028B;
        const IA32_MC12_CTL2                        = 0x0000028C;
        const IA32_MC13_CTL2                        = 0x0000028D;
        const IA32_MC14_CTL2                        = 0x0000028E;
        const IA32_MC15_CTL2                        = 0x0000028F;
        const IA32_MC16_CTL2                        = 0x00000290;
        const IA32_MC17_CTL2                        = 0x00000291;
        const IA32_MC18_CTL2                        = 0x00000292;
        const IA32_MC19_CTL2                        = 0x00000293;
        const IA32_MC20_CTL2                        = 0x00000294;
        const IA32_MC21_CTL2                        = 0x00000295;
        const IA32_MC22_CTL2                        = 0x00000296;
        const IA32_MC23_CTL2                        = 0x00000297;
        const IA32_MC24_CTL2                        = 0x00000298;
        const IA32_MC25_CTL2                        = 0x00000299;
        const IA32_MC26_CTL2                        = 0x0000029A;
        const IA32_MC27_CTL2                        = 0x0000029B;
        const IA32_MC28_CTL2                        = 0x0000029C;
        const IA32_MC29_CTL2                        = 0x0000029D;
        const IA32_MC30_CTL2                        = 0x0000029E;
        const IA32_MC31_CTL2                        = 0x0000029F;
        const IA32_MTRR_DEF_TYPE                    = 0x000002FF;
        const IA32_FIXED_CTR0                       = 0x00000309;
        const IA32_FIXED_CTR1                       = 0x0000030A;
        const IA32_FIXED_CTR2                       = 0x0000030B;
        const IA32_PERF_CAPABILITIES                = 0x00000345;
        const IA32_FIXED_CTR_CTRL                   = 0x0000038D;
        const IA32_PERF_GLOBAL_STATUS               = 0x0000038E;
        const IA32_PERF_GLOBAL_CTRL                 = 0x0000038F;
        const IA32_PERF_GLOBAL_OVF_CTRL             = 0x00000390;
        const IA32_PERF_GLOBAL_STATUS_RESET         = 0x00000390;
        const IA32_PERF_GLOBAL_STATUS_SET           = 0x00000391;
        const IA32_PERF_GLOBAL_INUSE                = 0x00000392;
        const IA32_PEBS_ENABLE                      = 0x000003F1;
        const IA32_MC0_CTL                          = 0x00000400;
        const IA32_MC0_STATUS                       = 0x00000401;
        const IA32_MC0_ADDR                         = 0x00000402;
        const IA32_MC0_MISC                         = 0x00000403;
        const IA32_MC1_CTL                          = 0x00000404;
        const IA32_MC1_STATUS                       = 0x00000405;
        const IA32_MC1_ADDR                         = 0x00000406;
        const IA32_MC1_MISC                         = 0x00000407;
        const IA32_MC2_CTL                          = 0x00000408;
        const IA32_MC2_STATUS                       = 0x00000409;
        const IA32_MC2_ADDR                         = 0x0000040A;
        const IA32_MC2_MISC                         = 0x0000040B;
        const IA32_MC3_CTL                          = 0x0000040C;
        const IA32_MC3_STATUS                       = 0x0000040D;
        const IA32_MC3_ADDR                         = 0x0000040E;
        const IA32_MC3_MISC                         = 0x0000040F;
        const IA32_MC4_CTL                          = 0x00000410;
        const IA32_MC4_STATUS                       = 0x00000411;
        const IA32_MC4_ADDR                         = 0x00000412;
        const IA32_MC4_MISC                         = 0x00000413;
        const IA32_MC5_CTL                          = 0x00000414;
        const IA32_MC5_STATUS                       = 0x00000415;
        const IA32_MC5_ADDR                         = 0x00000416;
        const IA32_MC5_MISC                         = 0x00000417;
        const IA32_MC6_CTL                          = 0x00000418;
        const IA32_MC6_STATUS                       = 0x00000419;
        const IA32_MC6_ADDR                         = 0x0000041A;
        const IA32_MC6_MISC                         = 0x0000041B;
        const IA32_MC7_CTL                          = 0x0000041C;
        const IA32_MC7_STATUS                       = 0x0000041D;
        const IA32_MC7_ADDR                         = 0x0000041E;
        const IA32_MC7_MISC                         = 0x0000041F;
        const IA32_MC8_CTL                          = 0x00000420;
        const IA32_MC8_STATUS                       = 0x00000421;
        const IA32_MC8_ADDR                         = 0x00000422;
        const IA32_MC8_MISC                         = 0x00000423;
        const IA32_MC9_CTL                          = 0x00000424;
        const IA32_MC9_STATUS                       = 0x00000425;
        const IA32_MC9_ADDR                         = 0x00000426;
        const IA32_MC9_MISC                         = 0x00000427;
        const IA32_MC10_CTL                         = 0x00000428;
        const IA32_MC10_STATUS                      = 0x00000429;
        const IA32_MC10_ADDR                        = 0x0000042A;
        const IA32_MC10_MISC                        = 0x0000042B;
        const IA32_MC11_CTL                         = 0x0000042C;
        const IA32_MC11_STATUS                      = 0x0000042D;
        const IA32_MC11_ADDR                        = 0x0000042E;
        const IA32_MC11_MISC                        = 0x0000042F;
        const IA32_MC12_CTL                         = 0x00000430;
        const IA32_MC12_STATUS                      = 0x00000431;
        const IA32_MC12_ADDR                        = 0x00000432;
        const IA32_MC12_MISC                        = 0x00000433;
        const IA32_MC13_CTL                         = 0x00000434;
        const IA32_MC13_STATUS                      = 0x00000435;
        const IA32_MC13_ADDR                        = 0x00000436;
        const IA32_MC13_MISC                        = 0x00000437;
        const IA32_MC14_CTL                         = 0x00000438;
        const IA32_MC14_STATUS                      = 0x00000439;
        const IA32_MC14_ADDR                        = 0x0000043A;
        const IA32_MC14_MISC                        = 0x0000043B;
        const IA32_MC15_CTL                         = 0x0000043C;
        const IA32_MC15_STATUS                      = 0x0000043D;
        const IA32_MC15_ADDR                        = 0x0000043E;
        const IA32_MC15_MISC                        = 0x0000043F;
        const IA32_MC16_CTL                         = 0x00000440;
        const IA32_MC16_STATUS                      = 0x00000441;
        const IA32_MC16_ADDR                        = 0x00000442;
        const IA32_MC16_MISC                        = 0x00000443;
        const IA32_MC17_CTL                         = 0x00000444;
        const IA32_MC17_STATUS                      = 0x00000445;
        const IA32_MC17_ADDR                        = 0x00000446;
        const IA32_MC17_MISC                        = 0x00000447;
        const IA32_MC18_CTL                         = 0x00000448;
        const IA32_MC18_STATUS                      = 0x00000449;
        const IA32_MC18_ADDR                        = 0x0000044A;
        const IA32_MC18_MISC                        = 0x0000044B;
        const IA32_MC19_CTL                         = 0x0000044C;
        const IA32_MC19_STATUS                      = 0x0000044D;
        const IA32_MC19_ADDR                        = 0x0000044E;
        const IA32_MC19_MISC                        = 0x0000044F;
        const IA32_MC20_CTL                         = 0x00000450;
        const IA32_MC20_STATUS                      = 0x00000451;
        const IA32_MC20_ADDR                        = 0x00000452;
        const IA32_MC20_MISC                        = 0x00000453;
        const IA32_MC21_CTL                         = 0x00000454;
        const IA32_MC21_STATUS                      = 0x00000455;
        const IA32_MC21_ADDR                        = 0x00000456;
        const IA32_MC21_MISC                        = 0x00000457;
        const IA32_MC22_CTL                         = 0x00000458;
        const IA32_MC22_STATUS                      = 0x00000459;
        const IA32_MC22_ADDR                        = 0x0000045A;
        const IA32_MC22_MISC                        = 0x0000045B;
        const IA32_MC23_CTL                         = 0x0000045C;
        const IA32_MC23_STATUS                      = 0x0000045D;
        const IA32_MC23_ADDR                        = 0x0000045E;
        const IA32_MC23_MISC                        = 0x0000045F;
        const IA32_MC24_CTL                         = 0x00000460;
        const IA32_MC24_STATUS                      = 0x00000461;
        const IA32_MC24_ADDR                        = 0x00000462;
        const IA32_MC24_MISC                        = 0x00000463;
        const IA32_MC25_CTL                         = 0x00000464;
        const IA32_MC25_STATUS                      = 0x00000465;
        const IA32_MC25_ADDR                        = 0x00000466;
        const IA32_MC25_MISC                        = 0x00000467;
        const IA32_MC26_CTL                         = 0x00000468;
        const IA32_MC26_STATUS                      = 0x00000469;
        const IA32_MC26_ADDR                        = 0x0000046A;
        const IA32_MC26_MISC                        = 0x0000046B;
        const IA32_MC27_CTL                         = 0x0000046C;
        const IA32_MC27_STATUS                      = 0x0000046D;
        const IA32_MC27_ADDR                        = 0x0000046E;
        const IA32_MC27_MISC                        = 0x0000046F;
        const IA32_MC28_CTL                         = 0x00000470;
        const IA32_MC28_STATUS                      = 0x00000471;
        const IA32_MC28_ADDR                        = 0x00000472;
        const IA32_MC28_MISC                        = 0x00000473;
        const IA32_VMX_BASIC                        = 0x00000480;
        const IA32_VMX_PINBASED_CTLS                = 0x00000481;
        const IA32_VMX_PROCBASED_CTLS               = 0x00000482;
        const IA32_VMX_EXIT_CTLS                    = 0x00000483;
        const IA32_VMX_ENTRY_CTLS                   = 0x00000484;
        const IA32_VMX_MISC                         = 0x00000485;
        const IA32_VMX_CR0_FIXED0                   = 0x00000486;
        const IA32_VMX_CR0_FIXED1                   = 0x00000487;
        const IA32_VMX_CR4_FIXED0                   = 0x00000488;
        const IA32_VMX_CR4_FIXED1                   = 0x00000489;
        const IA32_VMX_VMCS_ENUM                    = 0x0000048A;
        const IA32_VMX_PROCBASED_CTLS2              = 0x0000048B;
        const IA32_VMX_EPT_VPID_CAP                 = 0x0000048C;
        const IA32_VMX_TRUE_PINBASED_CTLS           = 0x0000048D;
        const IA32_VMX_TRUE_PROCBASED_CTLS          = 0x0000048E;
        const IA32_VMX_TRUE_EXIT_CTLS               = 0x0000048F;
        const IA32_VMX_TRUE_ENTRY_CTLS              = 0x00000490;
        const IA32_VMX_VMFUNC                       = 0x00000491;
        const IA32_VMX_PROCBASED_CTLS3              = 0x00000492;
        const IA32_VMX_EXIT_CTLS2                   = 0x00000493;
        const IA32_A_PMC0                           = 0x000004C1;
        const IA32_A_PMC1                           = 0x000004C2;
        const IA32_A_PMC2                           = 0x000004C3;
        const IA32_A_PMC3                           = 0x000004C4;
        const IA32_A_PMC4                           = 0x000004C5;
        const IA32_A_PMC5                           = 0x000004C6;
        const IA32_A_PMC6                           = 0x000004C7;
        const IA32_A_PMC7                           = 0x000004C8;
        const IA32_MCG_EXT_CTL                      = 0x000004D0;
        const IA32_SGX_SVN_STATUS                   = 0x00000500;
        const IA32_RTIT_OUTPUT_BASE                 = 0x00000560;
        const IA32_RTIT_OUTPUT_MASK_PTRS            = 0x00000561;
        const IA32_RTIT_CTL                         = 0x00000570;
        const IA32_RTIT_STATUS                      = 0x00000571;
        const IA32_RTIT_CR3_MATCH                   = 0x00000572;
        const IA32_RTIT_ADDR0_A                     = 0x00000580;
        const IA32_RTIT_ADDR0_B                     = 0x00000581;
        const IA32_RTIT_ADDR1_A                     = 0x00000582;
        const IA32_RTIT_ADDR1_B                     = 0x00000583;
        const IA32_RTIT_ADDR2_A                     = 0x00000584;
        const IA32_RTIT_ADDR2_B                     = 0x00000585;
        const IA32_RTIT_ADDR3_A                     = 0x00000586;
        const IA32_RTIT_ADDR3_B                     = 0x00000587;
        const IA32_DS_AREA                          = 0x00000600;
        const IA32_U_CET                            = 0x000006A0;
        const IA32_S_CET                            = 0x000006A2;
        const IA32_PL0_SSP                          = 0x000006A4;
        const IA32_PL1_SSP                          = 0x000006A5;
        const IA32_PL2_SSP                          = 0x000006A6;
        const IA32_PL3_SSP                          = 0x000006A7;
        const IA32_INTERRUPT_SSP_TABLE_ADDR         = 0x000006A8;
        const IA32_TSC_DEADLINE                     = 0x000006E0;
        const IA32_PKRS                             = 0x000006E1;
        const IA32_PM_ENABLE                        = 0x00000770;
        const IA32_HWP_CAPABILITIES                 = 0x00000771;
        const IA32_HWP_REQUEST_PKG                  = 0x00000772;
        const IA32_HWP_INTERRUPT                    = 0x00000773;
        const IA32_HWP_REQUEST                      = 0x00000774;
        const IA32_PECI_HWP_REQUEST_INFO            = 0x00000775;
        const IA32_HWP_CTL                          = 0x00000776;
        const IA32_HWP_STATUS                       = 0x00000777;
        const IA32_X2APIC_APICID                    = 0x00000802;
        const IA32_X2APIC_VERSION                   = 0x00000803;
        const IA32_X2APIC_TPR                       = 0x00000808;
        const IA32_X2APIC_PPR                       = 0x0000080A;
        const IA32_X2APIC_EOI                       = 0x0000080B;
        const IA32_X2APIC_LDR                       = 0x0000080D;
        const IA32_X2APIC_SIVR                      = 0x0000080F;
        const IA32_X2APIC_ISR0                      = 0x00000810;
        const IA32_X2APIC_ISR1                      = 0x00000811;
        const IA32_X2APIC_ISR2                      = 0x00000812;
        const IA32_X2APIC_ISR3                      = 0x00000813;
        const IA32_X2APIC_ISR4                      = 0x00000814;
        const IA32_X2APIC_ISR5                      = 0x00000815;
        const IA32_X2APIC_ISR6                      = 0x00000816;
        const IA32_X2APIC_ISR7                      = 0x00000817;
        const IA32_X2APIC_TMR0                      = 0x00000818;
        const IA32_X2APIC_TMR1                      = 0x00000819;
        const IA32_X2APIC_TMR2                      = 0x0000081A;
        const IA32_X2APIC_TMR3                      = 0x0000081B;
        const IA32_X2APIC_TMR4                      = 0x0000081C;
        const IA32_X2APIC_TMR5                      = 0x0000081D;
        const IA32_X2APIC_TMR6                      = 0x0000081E;
        const IA32_X2APIC_TMR7                      = 0x0000081F;
        const IA32_X2APIC_IRR0                      = 0x00000820;
        const IA32_X2APIC_IRR1                      = 0x00000821;
        const IA32_X2APIC_IRR2                      = 0x00000822;
        const IA32_X2APIC_IRR3                      = 0x00000823;
        const IA32_X2APIC_IRR4                      = 0x00000824;
        const IA32_X2APIC_IRR5                      = 0x00000825;
        const IA32_X2APIC_IRR6                      = 0x00000826;
        const IA32_X2APIC_IRR7                      = 0x00000827;
        const IA32_X2APIC_ESR                       = 0x00000828;
        const IA32_X2APIC_LVT_CMCI                  = 0x0000082F;
        const IA32_X2APIC_ICR                       = 0x00000830;
        const IA32_X2APIC_LVT_TIMER                 = 0x00000832;
        const IA32_X2APIC_LVT_THERMAL               = 0x00000833;
        const IA32_X2APIC_LVT_PMI                   = 0x00000834;
        const IA32_X2APIC_LVT_LINT0                 = 0x00000835;
        const IA32_X2APIC_LVT_LINT1                 = 0x00000836;
        const IA32_X2APIC_LVT_ERROR                 = 0x00000837;
        const IA32_X2APIC_INIT_COUNT                = 0x00000838;
        const IA32_X2APIC_CUR_COUNT                 = 0x00000839;
        const IA32_X2APIC_DIV_CONF                  = 0x0000083E;
        const IA32_X2APIC_SELF_IPI                  = 0x0000083F;
        const IA32_TME_CAPABILITY                   = 0x00000981;
        const IA32_TME_ACTIVATE                     = 0x00000982;
        const IA32_TME_EXCLUDE_MASK                 = 0x00000983;
        const IA32_TME_EXCLUDE_BASE                 = 0x00000984;
        const IA32_COPY_STATUS                      = 0x00000990;
        const IA32_IWKEYBACKUP_STATUS               = 0x00000991;
        const IA32_DEBUG_INTERFACE                  = 0x00000C80;
        const IA32_L3_QOS_CFG                       = 0x00000C81;
        const IA32_L2_QOS_CFG                       = 0x00000C82;
        const IA32_QM_EVTSEL                        = 0x00000C8D;
        const IA32_QM_CTR                           = 0x00000C8E;
        const IA32_PQR_ASSOC                        = 0x00000C8F;
        const IA32_L3_MASK_0                        = 0x00000C90;
        #[allow(non_upper_case_globals)]
        const IA32_L3_MASK_n                        = 0x00000C90;
        const IA32_L2_MASK_0                        = 0x00000D10;
        #[allow(non_upper_case_globals)]
        const IA32_L2_MASK_n                        = 0x00000D10;
        const IA32_BNDCFGS                          = 0x00000D90;
        const IA32_COPY_LOCAL_TO_PLATFORM           = 0x00000D91;
        const IA32_COPY_PLATFORM_TO_LOCAL           = 0x00000D92;
        const IA32_XSS                              = 0x00000DA0;
        const IA32_PKG_HDC_CTL                      = 0x00000DB0;
        const IA32_PM_CTL1                          = 0x00000DB1;
        const IA32_THREAD_STALL                     = 0x00000DB2;
        #[allow(non_upper_case_globals)]
        const IA32_LBR_x_INFO                       = 0x00001200;
        const IA32_LBR_CTL                          = 0x000014CE;
        const IA32_LBR_DEPTH                        = 0x000014CF;
        #[allow(non_upper_case_globals)]
        const IA32_LBR_x_FROM_IP                    = 0x00001500;
        #[allow(non_upper_case_globals)]
        const IA32_LBR_x_TO_IP                      = 0x00001600;
        const IA32_HW_FEEDBACK_PTR                  = 0x000017D0;
        const IA32_HW_FEEDBACK_CONFIG               = 0x000017D1;
        const IA32_HW_FEEDBACK_CHAR                 = 0x000017D2;
        const IA32_HW_FEEDBACK_THREAD_CONFIG        = 0x000017D4;
        const IA32_HRESET_ENABLE                    = 0x000017DA;
        const IA32_EFER                             = 0xC0000080;
        const IA32_STAR                             = 0xC0000081;
        const IA32_LSTAR                            = 0xC0000082;
        const IA32_CSTAR                            = 0xC0000083;
        const IA32_FMASK                            = 0xC0000084;
        const IA32_FS_BASE                          = 0xC0000100;
        const IA32_GS_BASE                          = 0xC0000101;
        const IA32_KERNEL_GS_BASE                   = 0xC0000102;
        const IA32_TSC_AUX                          = 0xC0000103;
    }
}
