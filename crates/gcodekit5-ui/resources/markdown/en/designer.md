# Designer

The designer has two distinct working modes: "2D Laser" and "3D CNC." You can access them in two ways: from the Designer's File menu by selecting "New 2D" or "New 3D," or by changing modes using the Machine Type selection in the left panel.

In 2D Laser mode, the Z coordinate is disabled. This mode is primarily used for laser engraving and cutting. You can create and edit objects using the built-in tools and import vector designs such as DXF and SVG. You can also import JPG, PNG, and other image formats for engraving.

<img src="../../help_images/designer.png" alt="Designer" width="700">

## Main Actions in 2D Laser Mode
- Draw primitives (rectangles, circles, lines, ellipses, polylines, triangles, polygons, pinions, gears)
- Add text
- Add images, even creating a composition of several images along with vector objects
- Independently define the engraving parameters for each image using its properties
- Define global parameters for vector objects that do not require independent parameters
- Independently define the parameters of each vector object using its properties
- Reorder the objects in the Objects panel, which will be used for G-code generation
- Import DXF and SVG files

- Out of Bounds warning. If any point goes outside the work area, an Out of Bounds warning will appear in the G-code so the user can determine what to do
- Export to G-code or SVG
- Generate the frame for material adjustment on the machine
- Generate the final G-code. When generating the G-code, you jump to the Visualizer tab to check the result before launching the job.

---
## Global Properties

<img src="../../help_images/global_properties.png" alt="Global Properties" width="600">

Clicking the "Tool Settings" button opens the window for global job settings. This configuration will be used for all vector objects that have the "Use Global Values" checkbox selected in the "Laser Settings (Object)" object properties.

---
## Individual Object Properties Panel

<img src="../../help_images/individual_properties.png" width="300">

In the right-hand panel, when an object is selected, its properties appear:
    <li>Position</li>
    <li>Size</li>
    <li>Rotation</li>
    <li>Corner (radius/rounding)</li>
    <li>Geometric Operations (offset, chamfer)</li>
    <li>CAM Properties</li>
    <li>Individual Laser Settings (speed, power, and passes) of a given object</li>

### Notes about Corner and Chamfer
- For polylines (open and closed), corner rounding is edited in the Corner panel using Radius.
- For Path objects, Corner Radius and Chamfer are mutually exclusive to avoid duplicated geometry.
- Chamfer is applied using real edge distance (for example, chamfer 10 means a 10 mm cut on each side of a 90-degree corner).

---
## Objects Panel

<img src="../../help_images/order_objects_1.png" width="300">
The object panel displays the list of objects with:
    <li> Order Number</li>
    <li> Object Type (Rectangle, Circle, Path, etc.) and ID #</li>
    <li> Object Name</li>
    <li> The order number is editable and is used to organize the objects when generating G-code so that the objects are executed in that order.</li>
    <li> The name is also editable, so that the objects can be conveniently identified.</li>
    <img src="../../help_images/order_objects_2" width="300">
<li> </li>

---
## Gcode and Frame Generator

<img src="../../help_images/gcode_frame.png" alt="Gcode and Frame Generator G-Code and Frame" width="200">

- In the Designer's left panel are the "Generate G-Code" and "Frame" buttons. After completing the design, it's advisable to generate the job's perimeter so you can send it to the machine and center the material properly. Once this process is complete, return to the Designer to generate the G-Code using the button. Once generated, you'll automatically jump to the Visualizer tab to see how the job will be executed. Once satisfied, go to Machine Control to start the job.
---

---
## Main Actions in 3D CNC Mode
- In this mode, the Z coordinate is enabled. **This coordinate is the dimension to which the tool will descend from the top face of the material.**
- Define global parameters for vector objects that do not require independent parameters using the "Stock Settings" and "Tool Settings" buttons. The first opens a dialog box with the material dimensions and the tool's safety height for non-operating movements. The second opens a dialog box to enter the travel speed, tool revolutions, diameter, and depth of cut.

- Independently define the working parameters for each object in the properties panel, "CAM Properties." When using object-specific values, the Z value is disregarded, and the Depth of Cut value from "CAM Properties" is used.

- The rest is the same as in 2D Mode

- **IMPORTANT:** Objects that do not have a Z dimension or have a Z dimension of 0 are drawn at the upper material dimension, since the Z dimension is always considered downwards in 3D mode.

## Related
[Visualizer](help:visualizer)
[Machine Control](help:machine_control)
[Index](help:index)
