package counter

import spinal.core._
import spinal.core.sim._

// Simulation that generates VCD waveform file
object CounterSim extends App {
  // Configure simulation with VCD output
  val simConfig = SimConfig
    .withWave                    // Enable waveform generation
    .withConfig(SpinalConfig(
      defaultConfigForClockDomains = ClockDomainConfig(resetKind = SYNC)
    ))

  simConfig.compile(Counter()).doSim { dut =>
    // Fork a clock generation process
    val clockPeriod = 10 // 10 time units = 100 MHz
    dut.clockDomain.forkStimulus(period = clockPeriod)

    // Initialize
    dut.io.enable #= false

    // Wait for reset
    dut.clockDomain.waitSampling(5)

    // Test 1: Enable counting
    println("Test 1: Enable counting")
    dut.io.enable #= true
    dut.clockDomain.waitSampling(20)

    // Test 2: Disable counting
    println("Test 2: Disable counting")
    dut.io.enable #= false
    dut.clockDomain.waitSampling(5)

    // Test 3: Resume counting
    println("Test 3: Resume counting")
    dut.io.enable #= true
    dut.clockDomain.waitSampling(10)

    // Test 4: Count to overflow (limited cycles for reasonable VCD size)
    println("Test 4: Counting to overflow")
    var cycles = 0
    var sawOverflow = false
    while (!sawOverflow && cycles < 260) {
      dut.clockDomain.waitSampling()
      cycles += 1
      if (dut.io.overflow.toBoolean) {
        sawOverflow = true
        println(s"  Overflow detected at cycle $cycles")
      }
    }

    // Continue a bit after overflow
    dut.clockDomain.waitSampling(5)

    println("Simulation complete!")
  }
}
