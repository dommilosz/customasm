use std::rc::Rc;
use crate::util;


#[derive(Debug, Clone, Hash, Eq)]
pub struct Span
{
	pub file: Rc<String>,
	pub location: Option<(usize, usize)>,
	pub contents: Option<String>
}


impl Span
{
	pub fn new(filename: Rc<String>, start: usize, end: usize) -> Span
	{
		Span
		{
			file: filename,
			location: Some((start, end)),
			contents: None
		}
	}

	pub fn new_noread(filename: Rc<String>, start: usize, end: usize) -> Span
	{
		Span
		{
			file: filename,
			location: Some((start, end)),
			contents: None
		}
	}

	pub fn new_loc(filename: Rc<String>, location:Option<(usize, usize)>) -> Span
	{
		Span
		{
			file: filename,
			location,
			contents: None
		}
	}

	pub fn read(&self, fileserver: &dyn util::FileServer) -> String {
		match self.contents.clone() {
			Some(contents)=>contents,
			None => fileserver.read_span(self)
		}
	}
	
	
	pub fn new_dummy() -> Span
	{
		Span
		{
			file: Rc::new("".to_string()),
			location: None,
			contents: None
		}
	}

	pub fn new_from_string(str:String) -> Span
	{
		Span
		{
			file: Rc::new("".to_string()),
			location: None,
			contents: Option::from(str)
		}
	}
	
	
	pub fn before(&self) -> Span
	{
		if self.location.is_none()
			{ self.clone() }
		
		else
		{
			let start = self.location.unwrap().0;
			Span::new(self.file.clone(),start,start)
		}
	}
	
	
	pub fn after(&self) -> Span
	{
		if self.location.is_none()
			{ self.clone() }
		
		else
		{
			let end = self.location.unwrap().1;
			Span::new(self.file.clone(),end,end)
		}
	}
	
	
	pub fn join(&self, other: &Span) -> Span
	{
		if self.location.is_none()
			{ return other.clone(); }
			
		else if other.location.is_none()
			{ return self.clone(); }
			
		assert!(self.file == other.file, "joining spans from different files");

		let location =
		{
			use std::cmp::{max, min};
			let start = min(self.location.unwrap().0, other.location.unwrap().0);
			let end   = max(self.location.unwrap().1, other.location.unwrap().1);
			Some((start, end))
		};

		Span::new_loc(self.file.clone(),location)
	}
}


impl PartialEq for Span
{
	fn eq(&self, other: &Self) -> bool
	{
		self.file == other.file && self.location == other.location
	}
}