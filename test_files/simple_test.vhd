-- Simple VHDL testbench for generating GHW waveform
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity simple_test is
end simple_test;

architecture tb of simple_test is
    signal clk : std_logic := '0';
    signal data : std_logic_vector(7 downto 0) := x"00";
    signal counter : integer range 0 to 255 := 0;
begin
    -- Clock process
    clk_process: process
    begin
        for i in 0 to 19 loop
            clk <= '0';
            wait for 10 ns;
            clk <= '1';
            wait for 10 ns;
        end loop;
        wait;
    end process;

    -- Data process
    data_process: process(clk)
    begin
        if rising_edge(clk) then
            counter <= counter + 1;
            data <= std_logic_vector(to_unsigned(counter, 8));
        end if;
    end process;
end tb;
