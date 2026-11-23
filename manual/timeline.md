# Timeline and Exposure Sheet

## Tween bars and keys

Motion tweens have a purple continuous bar; shape tweens have a green one. Diamonds identify keyed drawings within a span. Select a frame to inspect its evaluated artwork and edit its transform in Properties to create or change a key.

Drag the **first key** of a tween to relocate its entire span, including its drawings, keys and timing curve. A destination outline shows whether the drop is valid. Occupied drawing keys and other tweens are protected. If a neighboring tween shares an endpoint, its original boundary drawing is retained while the moved tween gets a copy.

To move internal keys, Shift-click to select a range and drag one of its keys. Relative spacing is preserved. A key represents the drawing/object state, including its keyed position, scale and rotation; separate property tracks are not displayed. Use Undo once to restore the move.

On **Rigging · Bone pose**, Shift-click a range and drag one of its diamonds to relocate the selected bone and pin keys together. Drops that collide with unselected keys or fall outside the timeline are rejected.

Continuous changes to object transforms, bone poses and pin positions in Properties form one undo step. Release the slider or confirm a number to finish; Escape during the edit restores the previous state.

## Shape Tween

Draw on a vector layer, then duplicate the drawing into a later keyframe. Use Pen or Node to reshape the copy while keeping the same nodes and drawing order. Right-click a frame between the keys and choose **Create Shape Tween**. Scrub the green span to preview the change. With artwork selected in the span, the timing graph in **Properties** adjusts its easing.

A keyframe starts an exposure. Held frames continue showing the same drawing. Use a blank keyframe to start a new drawing, or drag an exposure edge to change its duration.

Open the **Exposure Sheet** from the timeline. Each column is a layer and each row is a frame. Select cells to edit timing or classify drawings as keys, breakdowns, or in-betweens. Use **On ones**, **On twos**, or **On threes** to adjust cadence.

Select a keyframe range and choose **Transform keyframes** to move, rotate, or scale drawings together. Use **Shift & Trace** to align two temporary references while drawing the current frame.
