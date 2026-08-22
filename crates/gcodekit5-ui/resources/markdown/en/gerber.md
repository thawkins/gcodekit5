# Gerber to G-Code

The Gerber to G-Code tool allows you to convert standard Gerber files (commonly used for PCB manufacturing) into G-Code for CNC isolation routing.

## Usage

1.  **Select Directory**: Click "Browse..." to select the folder containing your Gerber files. The tool will automatically detect and map common layer files (e.g., Top Copper, Drill Holes).
2.  **Layer Type**: Select the layer you want to process from the dropdown. The detected file for that layer will be shown below.
3.  **Parameters**:
    *   **Board Width/Height**: Dimensions of your PCB stock.
    *   **Offset X/Y**: Shift the origin of the G-Code.
    *   **Feed Rate**: Cutting speed in mm/min.
    *   **Spindle Speed**: Spindle RPM.
    *   **Cut Depth**: Z-depth for the cut (negative for cutting into material).
    *   **Safe Z**: Retract height for rapid moves.
    *   **Tool Diameter**: Diameter of your engraving bit (V-bit or end mill).
    *   **Isolation Width**: Additional width to clear around traces.
    *   **Remove Excess Copper**: (Rubout) If checked, generates toolpaths to remove all copper not part of the traces.
4.  **Alignment Holes**:
    *   **Generate Alignment Holes**: Adds drilling operations for alignment pins.
    *   **Hole Diameter**: Diameter of the alignment holes.
    *   **Margin**: Distance from the board edge to the alignment holes.

## Output

Click **Generate G-Code** to create the toolpath. The result will be loaded into the G-Code Editor/Visualizer.
