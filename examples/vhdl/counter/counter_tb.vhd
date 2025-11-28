-- Testbench for 8-bit Counter
-- Generates a GHW waveform file for viewing in NovyWave

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity counter_tb is
end entity counter_tb;

architecture sim of counter_tb is
    -- Clock period: 10ns (100 MHz)
    constant CLK_PERIOD : time := 10 ns;

    signal clk      : std_logic := '0';
    signal reset    : std_logic := '0';
    signal enable   : std_logic := '0';
    signal count    : std_logic_vector(7 downto 0);
    signal overflow : std_logic;

    signal sim_done : boolean := false;
begin
    -- Instantiate the counter
    uut: entity work.counter
        port map (
            clk      => clk,
            reset    => reset,
            enable   => enable,
            count    => count,
            overflow => overflow
        );

    -- Clock generation
    clk_process: process
    begin
        while not sim_done loop
            clk <= '0';
            wait for CLK_PERIOD / 2;
            clk <= '1';
            wait for CLK_PERIOD / 2;
        end loop;
        wait;
    end process;

    -- Stimulus process
    stimulus: process
    begin
        -- Initial reset
        reset <= '1';
        enable <= '0';
        wait for 50 ns;

        -- Release reset, enable counting
        reset <= '0';
        enable <= '1';
        wait for 300 ns;

        -- Disable counting briefly
        enable <= '0';
        wait for 50 ns;

        -- Resume counting
        enable <= '1';
        wait for 200 ns;

        -- Apply reset while counting
        reset <= '1';
        wait for 30 ns;
        reset <= '0';
        wait for 200 ns;

        -- Continue until overflow
        wait for 3000 ns;

        -- End simulation
        sim_done <= true;
        wait;
    end process;

end architecture sim;
