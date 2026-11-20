use super::*;

pub(in crate::dex_run) fn java_collection_interface_method(method: &DexMethodId) -> bool {
    matches!(
        method.class_descriptor.as_str(),
        "Ljava/lang/Iterable;"
            | "Ljava/util/Collection;"
            | "Ljava/util/Iterator;"
            | "Ljava/util/List;"
            | "Ljava/util/ListIterator;"
            | "Ljava/util/Map;"
            | "Ljava/util/Set;"
    )
}

pub(in crate::dex_run) fn java_lang_object_virtual_method(method: &DexMethodId) -> bool {
    method.class_descriptor == "Ljava/lang/Object;"
        && matches!(
            (method.name.as_str(), method.proto.descriptor().as_str()),
            ("equals", "(Ljava/lang/Object;)Z")
                | ("hashCode", "()I")
                | ("toString", "()Ljava/lang/String;")
        )
}

pub(in crate::dex_run) fn java_lang_object_equals_method(method: &DexMethodId) -> bool {
    method.class_descriptor == "Ljava/lang/Object;"
        && method.name == "equals"
        && method.proto.descriptor_is("(Ljava/lang/Object;)Z")
}

pub(in crate::dex_run) fn is_menu_inflater_inflate_method(method: &DexMethodId) -> bool {
    method.name == "inflate" && method.proto.descriptor_is("(ILandroid/view/Menu;)V")
}

pub(in crate::dex_run) fn records_content_view_method(method: &DexMethodId) -> bool {
    method.name == "setContentView"
        && matches!(
            method.proto.descriptor().as_str(),
            "(Landroid/view/View;)V"
                | "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"
        )
}

pub(in crate::dex_run) fn activity_set_content_view_runtime_view_method(
    method: &DexMethodId,
) -> bool {
    method.class_descriptor == "Landroid/app/Activity;"
        && method.name == "setContentView"
        && matches!(
            method.proto.descriptor().as_str(),
            "(Landroid/view/View;)V"
                | "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"
        )
}

// cite: frameworks-base Dialog.java#show
// cite: frameworks-base WindowManagerGlobal.java#addView
pub(in crate::dex_run) fn dialog_show_runtime_view_method(method: &DexMethodId) -> bool {
    method.class_descriptor == "Landroid/app/Dialog;"
        && method.name == "show"
        && method.proto.descriptor_is("()V")
}

// cite: frameworks-base Dialog.java#dismiss, dismissDialog
pub(in crate::dex_run) fn dialog_dismiss_runtime_view_method(method: &DexMethodId) -> bool {
    method.class_descriptor == "Landroid/app/Dialog;"
        && method.name == "dismiss"
        && method.proto.descriptor_is("()V")
}

pub(in crate::dex_run) fn adapter_view_set_adapter_method(method: &DexMethodId) -> bool {
    matches!(
        method.class_descriptor.as_str(),
        "Landroid/widget/AdapterView;"
            | "Landroid/widget/AbsListView;"
            | "Landroid/widget/ListView;"
            | "Landroid/widget/GridView;"
    ) && method.name == "setAdapter"
        && matches!(
            method.proto.descriptor().as_str(),
            "(Landroid/widget/ListAdapter;)V"
        )
}

pub(in crate::dex_run) fn type_shorty(type_descriptor: &str) -> char {
    match type_descriptor.chars().next() {
        Some('L') | Some('[') => 'L',
        Some(code) => code,
        None => 'V',
    }
}

pub(in crate::dex_run) fn view_group_add_view_runtime_child_method(method: &DexMethodId) -> bool {
    method.class_descriptor == "Landroid/view/ViewGroup;"
        && method.name == "addView"
        && matches!(
            method.proto.descriptor().as_str(),
            "(Landroid/view/View;)V"
                | "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"
                | "(Landroid/view/View;I)V"
                | "(Landroid/view/View;ILandroid/view/ViewGroup$LayoutParams;)V"
        )
}

pub(in crate::dex_run) fn view_group_remove_all_views_method(method: &DexMethodId) -> bool {
    method.class_descriptor == "Landroid/view/ViewGroup;"
        && method.name == "removeAllViews"
        && method.proto.descriptor_is("()V")
}

pub(in crate::dex_run) fn view_group_draw_child_method(method: &DexMethodId) -> bool {
    method.class_descriptor == "Landroid/view/ViewGroup;"
        && method.name == "drawChild"
        && method
            .proto
            .descriptor_is("(Landroid/graphics/Canvas;Landroid/view/View;J)Z")
}

pub(in crate::dex_run) fn view_group_should_delay_child_pressed_state_method(
    method: &DexMethodId,
) -> bool {
    matches!(
        method.class_descriptor.as_str(),
        "Landroid/view/ViewGroup;"
            | "Landroid/widget/FrameLayout;"
            | "Landroid/widget/LinearLayout;"
            | "Landroid/widget/RelativeLayout;"
            | "Landroid/widget/ScrollView;"
            | "Landroid/widget/HorizontalScrollView;"
    ) && method.name == "shouldDelayChildPressedState"
        && method.proto.descriptor_is("()Z")
}

pub(in crate::dex_run) fn arrays_sort_object_range_method(method: &DexMethodId) -> bool {
    method.class_descriptor == "Ljava/util/Arrays;"
        && method.name == "sort"
        && method
            .proto
            .descriptor_is("([Ljava/lang/Object;IILjava/util/Comparator;)V")
}

pub(in crate::dex_run) fn arrays_sort_object_with_comparator_method(method: &DexMethodId) -> bool {
    method.class_descriptor == "Ljava/util/Arrays;"
        && method.name == "sort"
        && method
            .proto
            .descriptor_is("([Ljava/lang/Object;Ljava/util/Comparator;)V")
}

pub(in crate::dex_run) fn collections_sort_list_with_comparator_method(
    method: &DexMethodId,
) -> bool {
    method.class_descriptor == "Ljava/util/Collections;"
        && method.name == "sort"
        && method
            .proto
            .descriptor_is("(Ljava/util/List;Ljava/util/Comparator;)V")
}
