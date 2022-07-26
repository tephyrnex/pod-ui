use anyhow::*;
use gtk::prelude::*;
use glib::Object;
use gtk::{Builder, Widget};
use std::iter::Iterator;
use std::ops::Add;

pub struct ObjectList {
    objects: Vec<Object>
}

impl ObjectList {
    pub fn new(builder: &Builder) -> Self {
        ObjectList { objects: builder.objects() }
    }

    pub fn obj_by_name(&self, name: &str) -> Result<Object> {
        self.objects.iter()
            .find(|o| ObjectList::object_name(*o).unwrap_or("".into()) == name)
            .with_context(|| format!("Object not found by name {:?}", name))
            .map(|obj| obj.clone())
    }

    pub fn ref_by_name<T: ObjectType>(&self, name: &str) -> Result<T> {
        let obj = self.obj_by_name(name)?;
        let cast = obj.dynamic_cast_ref::<T>()
            .with_context(|| format!("Object by name {:?} is can not be cast to type {:?}", name, T::static_type()))?
            .clone();
        Ok(cast)
    }

    pub fn named_objects(&self) -> impl Iterator<Item=(&Object, String)> {
        self.objects.iter()
            .flat_map(|obj| ObjectList::object_name(obj).map(|name| (obj, name)))
    }

    pub fn object_name<T: ObjectType>(obj: &T) -> Option<String> {
        obj.try_property::<String>("name")
            .ok()
            .filter(|v| !v.is_empty())
    }

    pub fn widgets_by_class_match<'a, F>(&'a self, filter: &'a F) -> impl Iterator<Item=(&Widget, Vec<String>)>
        where
            F: Fn(&String) -> bool
    {
        self.objects.iter()
            .filter_map(|obj| obj.dynamic_cast_ref::<Widget>())
            .filter_map(move |widget| {
                let style_context = widget.style_context();
                let classes = style_context.list_classes();
                let classes = classes.iter().map(|p| p.as_str().to_string());
                let m = classes.filter(filter).collect::<Vec<_>>();
                if !m.is_empty() { Some((widget, m)) } else { None }
            })
    }

    pub fn dump_debug(&self) {
        println!("object list debug ---");
        self.objects.iter()
            .for_each(|obj| {
                let type_name = obj.type_().name();
                let name = ObjectList::object_name(obj).map(|n| format!("{:?} ", n)).unwrap_or_default();
                println!("{} {}{{", type_name, name);
                /*
                let props = obj.list_properties();
                for p in props {
                    let p_name = p.get_name();
                    let p_type = p.get_value_type().name();
                    println!("  - {} '{}'", p_type, p_name);
                }
                 */
                println!("}}");
                /*
                let sc = obj.dynamic_cast_ref::<gtk::Widget>().map(|x| x.style_context()).unwrap_or_default();
                let cc = sc.list_classes();
                let ss = cc.iter().map(|p| p.to_string()).collect::<Vec<_>>();
                //println!("{:?}", ss);

                //let s: gtk_sys::Style = obj.get_property("style").map(|p| p.get().unwrap().unwrap()).unwrap();
                //println!("{:?}", s)
                */
            });
    }
}

impl Default for ObjectList {
    fn default() -> Self {
        ObjectList { objects: Vec::default() }
    }
}

impl Clone for ObjectList {
    fn clone(&self) -> Self {
        ObjectList {
            objects: self.objects.clone()
        }
    }
}

impl Add<ObjectList> for ObjectList {
    type Output = ObjectList;

    fn add(self, rhs: ObjectList) -> Self::Output {
        let mut out = self.clone();
        let mut rhs = rhs.objects.clone();
        out.objects.append(&mut rhs);

        out
    }
}