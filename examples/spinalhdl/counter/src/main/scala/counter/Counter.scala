package counter

import spinal.core._
import spinal.lib._

// 8-bit Counter with enable and overflow detection
case class Counter() extends Component {
  val io = new Bundle {
    val enable   = in  Bool()
    val count    = out UInt(8 bits)
    val overflow = out Bool()
  }

  // Internal counter register
  val counterReg = Reg(UInt(8 bits)) init(0)

  // Counter logic
  when(io.enable) {
    counterReg := counterReg + 1
  }

  // Overflow detection (counter about to wrap)
  val overflowReg = Reg(Bool()) init(False)
  when(io.enable && counterReg === 255) {
    overflowReg := True
  } otherwise {
    overflowReg := False
  }

  // Output assignments
  io.count    := counterReg
  io.overflow := overflowReg
}

// Generate Verilog RTL
object CounterVerilog extends App {
  SpinalConfig(
    targetDirectory = "rtl",
    defaultConfigForClockDomains = ClockDomainConfig(resetKind = SYNC)
  ).generateVerilog(Counter())
}

// Generate VHDL RTL
object CounterVhdl extends App {
  SpinalConfig(
    targetDirectory = "rtl",
    defaultConfigForClockDomains = ClockDomainConfig(resetKind = SYNC)
  ).generateVhdl(Counter())
}
