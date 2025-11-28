-- Simple 8-bit Counter for NovyWave Demonstration
-- This example generates a GHW waveform file using GHDL

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity counter is
    port (
        clk     : in  std_logic;
        reset   : in  std_logic;
        enable  : in  std_logic;
        count   : out std_logic_vector(7 downto 0);
        overflow: out std_logic
    );
end entity counter;

architecture rtl of counter is
    signal count_reg : unsigned(7 downto 0) := (others => '0');
begin
    process(clk, reset)
    begin
        if reset = '1' then
            count_reg <= (others => '0');
            overflow <= '0';
        elsif rising_edge(clk) then
            if enable = '1' then
                if count_reg = 255 then
                    count_reg <= (others => '0');
                    overflow <= '1';
                else
                    count_reg <= count_reg + 1;
                    overflow <= '0';
                end if;
            end if;
        end if;
    end process;

    count <= std_logic_vector(count_reg);
end architecture rtl;
