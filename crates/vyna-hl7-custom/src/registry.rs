//! Global registry for custom Z-segments

use crate::error::{CustomSegmentError, Result};
use crate::segment::CustomSegment;
use once_cell::sync::Lazy;
use std::any::Any;
use std::collections::HashMap;
use std::sync::RwLock;
use vyna_hl7_core::Segment;

/// Factory trait for creating custom segments from generic segments
///
/// This trait is implemented automatically when registering a CustomSegment
/// and enables dynamic dispatch for segment creation.
pub trait CustomSegmentFactory: Send + Sync {
    /// Create a boxed custom segment from a generic segment
    fn create(&self, segment: &Segment) -> Result<Box<dyn Any + Send>>;

    /// Get the segment ID this factory handles
    fn segment_id(&self) -> &'static str;

    /// Get the type name for debugging
    fn type_name(&self) -> &'static str;
}

/// Information about a registered segment factory
#[derive(Debug, Clone)]
pub struct FactoryInfo {
    /// The segment ID this factory handles (e.g., "ZPV")
    pub segment_id: String,
    /// The Rust type name for debugging
    pub type_name: String,
}

/// Implementation of CustomSegmentFactory for types implementing CustomSegment
struct CustomSegmentFactoryImpl<T: CustomSegment + 'static> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: CustomSegment + 'static> CustomSegmentFactoryImpl<T> {
    fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: CustomSegment + 'static> CustomSegmentFactory for CustomSegmentFactoryImpl<T> {
    fn create(&self, segment: &Segment) -> Result<Box<dyn Any + Send>> {
        let custom_segment = T::from_segment(segment)?;
        custom_segment.validate()?;
        Ok(Box::new(custom_segment))
    }

    fn segment_id(&self) -> &'static str {
        T::segment_id()
    }

    fn type_name(&self) -> &'static str {
        T::type_name()
    }
}

/// Global registry for custom Z-segments
///
/// This registry maintains a mapping of segment IDs to their factories,
/// enabling dynamic creation and parsing of custom segments.
///
/// # Thread Safety
///
/// The registry is thread-safe and can be accessed from multiple threads
/// concurrently. Registration should typically happen during application
/// startup before parsing begins.
///
/// # Example
///
/// ```rust,ignore
/// use vyna_hl7_custom::{CustomSegment, CustomSegmentRegistry};
///
/// // Register custom segments at startup (`register` returns a
/// // `Result`, so each call handles its own error).
/// fn init() -> vyna_hl7_custom::error::Result<()> {
///     CustomSegmentRegistry::global().register::<ZPV>()?;
///     CustomSegmentRegistry::global().register::<ZCU>()?;
///     Ok(())
/// }
///
/// // Later, check if a segment is custom
/// if CustomSegmentRegistry::global().is_registered("ZPV") {
///     println!("ZPV is a registered custom segment");
/// }
/// ```
pub struct CustomSegmentRegistry {
    factories: RwLock<HashMap<String, Box<dyn CustomSegmentFactory>>>,
}

impl CustomSegmentRegistry {
    /// Create a new empty registry
    ///
    /// This is primarily useful for testing. In production code, use
    /// [`CustomSegmentRegistry::global()`] to access the singleton instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vyna_hl7_custom::CustomSegmentRegistry;
    ///
    /// let registry = CustomSegmentRegistry::new();
    /// // Use for isolated testing...
    /// ```
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global singleton registry instance
    ///
    /// This is the primary way to access the registry. The instance is
    /// created lazily on first access.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vyna_hl7_custom::CustomSegmentRegistry;
    ///
    /// let registry = CustomSegmentRegistry::global();
    /// ```
    pub fn global() -> &'static Self {
        static REGISTRY: Lazy<CustomSegmentRegistry> = Lazy::new(CustomSegmentRegistry::new);
        &REGISTRY
    }

    /// Register a custom segment type
    ///
    /// # Arguments
    ///
    /// * `T` - The custom segment type implementing `CustomSegment`
    ///
    /// # Returns
    ///
    /// A shared reference to the registry for method chaining on success
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The segment ID is already registered (duplicate registration)
    /// - The segment ID is invalid (must be 'Z' plus 2 ASCII alphanumerics)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// CustomSegmentRegistry::global().register::<ZPV>()?;
    /// CustomSegmentRegistry::global().register::<ZCU>()?;
    /// ```
    pub fn register<T: CustomSegment + 'static>(&self) -> Result<&Self> {
        let segment_id = T::segment_id();

        // Validate segment ID: 'Z' plus 2 ASCII alphanumerics, the
        // same predicate `z_segment!` enforces at expansion, so
        // to_segment() output always passes Segment::validate_id.
        if !crate::macros::__is_z_segment_id(segment_id) {
            return Err(CustomSegmentError::InvalidSegmentId(segment_id.to_string()));
        }

        let mut factories = self.factories.write().map_err(|e| {
            CustomSegmentError::Other(format!("Failed to acquire write lock: {}", e))
        })?;

        // Check for duplicate registration
        if factories.contains_key(segment_id) {
            return Err(CustomSegmentError::DuplicateRegistration(
                segment_id.to_string(),
            ));
        }

        factories.insert(
            segment_id.to_string(),
            Box::new(CustomSegmentFactoryImpl::<T>::new()),
        );

        Ok(self)
    }

    /// Get metadata about a registered segment factory
    ///
    /// Returns the segment ID and type name for a registered factory.
    /// To create a custom segment instance, use [`parse_segment()`](Self::parse_segment) instead.
    ///
    /// # Arguments
    ///
    /// * `id` - The segment ID to look up (e.g., "ZPV")
    ///
    /// # Returns
    ///
    /// `Some(FactoryInfo)` if the segment is registered, `None` otherwise.
    /// A poisoned registry lock also yields `None` (indistinguishable from
    /// "not registered") rather than panicking; use the fallible
    /// [`create`](Self::create)/[`parse_segment`](Self::parse_segment)
    /// paths when a poisoned lock must surface as an error.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(info) = registry.get("ZPV") {
    ///     println!("Found factory for {} (type: {})", info.segment_id, info.type_name);
    /// }
    /// ```
    pub fn get(&self, id: &str) -> Option<FactoryInfo> {
        let factories = self.factories.read().ok()?;

        factories.get(id).map(|factory| FactoryInfo {
            segment_id: factory.segment_id().to_string(),
            type_name: factory.type_name().to_string(),
        })
    }

    /// Create a custom segment from a generic segment using the registered factory
    ///
    /// This is the primary way to use a registered factory. It acquires the
    /// read lock, looks up the factory, and creates the segment in one step.
    ///
    /// # Arguments
    ///
    /// * `id` - The segment ID to look up (e.g., "ZPV")
    /// * `segment` - The generic segment to convert
    ///
    /// # Returns
    ///
    /// A boxed custom segment if the ID is registered and creation succeeds,
    /// `None` if the segment isn't registered
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(custom) = registry.create("ZPV", &generic_segment)? {
    ///     // custom is Box<dyn Any + Send>
    ///     let zpv = custom.downcast::<ZPV>().unwrap();
    /// }
    /// ```
    pub fn create(&self, id: &str, segment: &Segment) -> Result<Option<Box<dyn Any + Send>>> {
        let factories = self.factories.read().map_err(|e| {
            CustomSegmentError::Other(format!("Failed to acquire read lock: {}", e))
        })?;

        if let Some(factory) = factories.get(id) {
            if segment.id != id {
                return Err(CustomSegmentError::parse_error(
                    id,
                    format!("expected {} segment, got {}", id, segment.id),
                ));
            }
            let custom_segment = factory.create(segment)?;
            Ok(Some(custom_segment))
        } else {
            Ok(None)
        }
    }

    /// Check if a segment ID is registered
    ///
    /// # Arguments
    ///
    /// * `id` - The segment ID to check
    ///
    /// # Returns
    ///
    /// `true` if the segment is registered, `false` otherwise. A poisoned
    /// registry lock yields `false` rather than panicking.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if registry.is_registered("ZPV") {
    ///     println!("ZPV is registered");
    /// }
    /// ```
    pub fn is_registered(&self, id: &str) -> bool {
        self.factories
            .read()
            .map(|f| f.contains_key(id))
            .unwrap_or(false)
    }

    /// Get a list of all registered segment IDs
    ///
    /// # Returns
    ///
    /// A vector of registered segment IDs, or an empty vector when the
    /// registry lock is poisoned (rather than panicking).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ids = registry.registered_ids();
    /// for id in ids {
    ///     println!("Registered: {}", id);
    /// }
    /// ```
    pub fn registered_ids(&self) -> Vec<String> {
        self.factories
            .read()
            .map(|f| f.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Parse a segment using its registered factory
    ///
    /// # Arguments
    ///
    /// * `segment` - The generic segment to parse
    ///
    /// # Returns
    ///
    /// A boxed custom segment if the ID is registered and parsing succeeds,
    /// `None` if the segment isn't registered
    ///
    /// # Errors
    ///
    /// Returns an error if parsing or validation fails
    pub fn parse_segment(&self, segment: &Segment) -> Result<Option<Box<dyn Any + Send>>> {
        let factories = self.factories.read().map_err(|e| {
            CustomSegmentError::Other(format!("Failed to acquire read lock: {}", e))
        })?;

        if let Some(factory) = factories.get(&segment.id) {
            let custom_segment = factory.create(segment)?;
            Ok(Some(custom_segment))
        } else {
            Ok(None)
        }
    }

    /// Clear all registered segments (mainly for testing)
    ///
    /// Does nothing when the registry lock is poisoned.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// registry.clear();
    /// assert_eq!(registry.registered_ids().len(), 0);
    /// ```
    #[cfg(test)]
    pub fn clear(&self) {
        if let Ok(mut factories) = self.factories.write() {
            factories.clear();
        }
    }
}

impl Default for CustomSegmentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::Segment;

    // Test Z-segment
    struct TestZPV {
        visit_type: String,
    }

    impl CustomSegment for TestZPV {
        fn segment_id() -> &'static str {
            "ZPV"
        }

        fn from_segment(segment: &Segment) -> Result<Self> {
            let visit_type = segment
                .get_field_value(1)
                .map(|s| s.to_string())
                .ok_or_else(|| CustomSegmentError::missing_field("ZPV-1", "ZPV"))?;

            Ok(TestZPV { visit_type })
        }

        fn to_segment(&self) -> Segment {
            let mut segment = Segment::new("ZPV");
            let _ = segment.set_field_value(1, &self.visit_type);
            segment
        }
    }

    #[test]
    fn test_register_segment() {
        let registry = CustomSegmentRegistry::new();
        let result = registry.register::<TestZPV>();
        assert!(result.is_ok());
        assert!(registry.is_registered("ZPV"));
    }

    #[test]
    fn test_duplicate_registration() {
        let registry = CustomSegmentRegistry::new();
        registry.register::<TestZPV>().unwrap();
        let result = registry.register::<TestZPV>();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_segment_id() {
        struct InvalidSegment;
        impl CustomSegment for InvalidSegment {
            fn segment_id() -> &'static str {
                "ABC" // Doesn't start with Z
            }
            fn from_segment(_: &Segment) -> Result<Self> {
                Ok(InvalidSegment)
            }
            fn to_segment(&self) -> Segment {
                Segment::new("ABC")
            }
        }

        let registry = CustomSegmentRegistry::new();
        let result = registry.register::<InvalidSegment>();
        assert!(result.is_err());
    }

    #[test]
    fn test_register_rejects_malformed_z_ids() {
        // Ids that fail Segment::validate_id ("Z", "ZPVX", "Z!") must
        // fail registration too: to_segment() output would otherwise
        // fail validation downstream.
        macro_rules! bad_segment {
            ($name:ident, $id:literal) => {
                struct $name;
                impl CustomSegment for $name {
                    fn segment_id() -> &'static str {
                        $id
                    }
                    fn from_segment(_: &Segment) -> Result<Self> {
                        Ok($name)
                    }
                    fn to_segment(&self) -> Segment {
                        Segment::new($id)
                    }
                }
            };
        }
        bad_segment!(ShortZ, "Z");
        bad_segment!(LongZ, "ZPVX");
        bad_segment!(PunctZ, "Z!");
        let registry = CustomSegmentRegistry::new();
        assert!(registry.register::<ShortZ>().is_err());
        assert!(registry.register::<LongZ>().is_err());
        assert!(registry.register::<PunctZ>().is_err());
        assert!(registry.registered_ids().is_empty());
        // A well-formed Z id still registers.
        assert!(registry.register::<TestZPV>().is_ok());
    }

    #[test]
    fn test_registered_ids() {
        let registry = CustomSegmentRegistry::new();
        registry.register::<TestZPV>().unwrap();

        let ids = registry.registered_ids();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&"ZPV".to_string()));
    }

    #[test]
    fn test_parse_segment() {
        let registry = CustomSegmentRegistry::new();
        registry.register::<TestZPV>().unwrap();

        let mut segment = Segment::new("ZPV");
        segment.set_field_value(1, "OUTPATIENT").unwrap();

        let result = registry.parse_segment(&segment);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_parse_unregistered_segment() {
        let registry = CustomSegmentRegistry::new();

        let segment = Segment::new("ZXY");
        let result = registry.parse_segment(&segment);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_create_rejects_mismatched_segment_id() {
        // A PID segment must not be created as ZPV: fields would bind to
        // the wrong positions.
        let registry = CustomSegmentRegistry::new();
        registry.register::<TestZPV>().unwrap();

        let mut pid = Segment::new("PID");
        pid.set_field_value(1, "1").unwrap();
        let err = registry.create("ZPV", &pid).unwrap_err();
        assert!(err.to_string().contains("expected ZPV"), "{err}");

        let mut zpv = Segment::new("ZPV");
        zpv.set_field_value(1, "OUTPATIENT").unwrap();
        assert!(registry.create("ZPV", &zpv).unwrap().is_some());
    }

    #[test]
    fn test_get_factory_info() {
        let registry = CustomSegmentRegistry::new();
        registry.register::<TestZPV>().unwrap();

        let info = registry.get("ZPV");
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.segment_id, "ZPV");

        // Unregistered segment returns None
        assert!(registry.get("ZXX").is_none());
    }

    #[test]
    fn test_lock_poison_degrades_gracefully() {
        let registry = CustomSegmentRegistry::new();
        registry.register::<TestZPV>().unwrap();

        // Poison the lock by panicking while holding the write guard.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = registry.factories.write().unwrap();
            panic!("intentional poison");
        }));

        // Documented behavior: infallible reads degrade to "not
        // registered" instead of panicking...
        assert!(registry.get("ZPV").is_none());
        assert!(!registry.is_registered("ZPV"));
        assert!(registry.registered_ids().is_empty());

        // ...while fallible paths surface the poison as an error.
        let segment = Segment::new("ZPV");
        assert!(registry.create("ZPV", &segment).is_err());
        assert!(registry.parse_segment(&segment).is_err());
        assert!(registry.register::<TestZPV>().is_err());
    }

    #[test]
    fn test_create_segment() {
        let registry = CustomSegmentRegistry::new();
        registry.register::<TestZPV>().unwrap();

        let mut segment = Segment::new("ZPV");
        segment.set_field_value(1, "INPATIENT").unwrap();

        let result = registry.create("ZPV", &segment);
        assert!(result.is_ok());
        let custom = result.unwrap();
        assert!(custom.is_some());

        // Downcast to verify the actual type
        let zpv = custom.unwrap().downcast::<TestZPV>().unwrap();
        assert_eq!(zpv.visit_type, "INPATIENT");

        // Unregistered ID returns None
        let result = registry.create("ZXX", &segment);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    struct TestZCU {
        balance: String,
    }

    impl CustomSegment for TestZCU {
        fn segment_id() -> &'static str {
            "ZCU"
        }

        fn from_segment(segment: &Segment) -> Result<Self> {
            let balance = segment
                .get_field_value(1)
                .map(|s| s.to_string())
                .ok_or_else(|| CustomSegmentError::missing_field("ZCU-1", "ZCU"))?;

            Ok(TestZCU { balance })
        }

        fn to_segment(&self) -> Segment {
            let mut segment = Segment::new("ZCU");
            let _ = segment.set_field_value(1, &self.balance);
            segment
        }
    }

    #[test]
    fn test_register_chains_through_unwrapped_shared_reference() {
        // `register` takes `&self` and returns `Result<&Self>`: chaining
        // goes through the unwrapped shared reference, and the return
        // type pins the shared (not mutable) borrow.
        let registry = CustomSegmentRegistry::new();
        let shared: &CustomSegmentRegistry = registry.register::<TestZPV>().unwrap();
        shared.register::<TestZCU>().unwrap();
        assert!(registry.is_registered("ZPV"));
        assert!(registry.is_registered("ZCU"));
    }
}
