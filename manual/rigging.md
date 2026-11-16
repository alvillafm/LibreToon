# Symbols, Clips and Rigging

A symbol is reusable artwork. Double-click an instance to edit its source. Use a clip to reuse an animation range. Select the range, choose **Create Animation Clip**, then drag the saved clip from **Animation** onto the timeline or stage.

Use **Bones** to create a hierarchy, bind symbol instances, and adjust weights. Use **Puppet** pins for localized deformation. In **Animation → Rig poses**, save poses you want to reuse.

## Head and body turns

1. Build a rig with a torso root and a head branch. Pose the two endpoints and save each under **Animation → Rig poses**.
2. Select **Bones → Set Up Character**. Under **Head turn**, choose the head root and assign a **Front** pose and at least one side pose.
3. Under **Body turn**, choose the torso root and assign its views. The body blend skips the head branch. The head follows its parent hierarchy.
4. Close Character Setup. Select a bound symbol with **Selection** and drag a turn control on the stage. This creates a key at the playhead. Expand **Rigging** in the timeline to move or delete keys for each turn.

Turns blend the rig's bone transforms. Draw separate view artwork when the silhouette or features need to change with the angle.

## Character Tools

To blend between adjacent character views, copy the first view into the next one and reshape the copy with Pen or Node. Right-click the first cell and enable **Interpolate Shape**. A green arrow marks the transition. Turning the character blends compatible drawings of the active expression or mouth pose. Hidden pieces and incompatible drawings use the regular view switching rules.

Select a bound piece on the stage and click **Edit Character Symbol** in the floating panel above the timeline. Inside the symbol, **Character Tools** floats above the view grid. Switch between **Artwork** and **Edit Bones**, or use **Copy from View**. Drag the view grid's top edge to resize the timeline. The onion-skin controls are directly above the view names.

Choose **Add Rigged Layer → Eye** above the view grid. Select a cell in **Open**, **Half closed**, or **Closed** and draw the eye for that angle. To interpolate, copy an eye drawing from another state in the cell menu, reshape its nodes, and enable **Vertical Shape Tween**. If the middle drawing is empty, the tween can interpolate directly between Open and Closed. In the scene, select the character and use **Eye closure** in **Face Rig** to key the eyelid.

Enable **Horizontal Shape Tween** from the first view of a transition to blend into the next drawn view at the current eyelid closure. Blank views are skipped. Double-click a row name to rename it, then press Enter. Renaming artwork linked to one bone also renames that bone. Use the arrow next to an eye to fold its three drawing rows.

Choose **Add Rigged Layer → Pupil**, draw the pupil for each angle, and use **Area Tool** to drag from the center of its travel circle to the edge. Back on the stage, drag the dot in the pupil's circular control in **Face Rig**. To draw an eye over its pupil, right-click the pupil row and use **Move up** until it appears before the eye in the drawing order.

## Lip sync

Create a Lip Sync symbol and draw its mouth poses. Add a dialogue clip, then open its right-click lip sync command and choose the intended symbol instance. Review and edit the generated mouth keys on the timeline.
