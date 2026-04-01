use embedded_hal::{
    digital,
    spi::{self, Operation},
};

pub trait Comms {
    type Error;

    fn set_cs(&self, active: bool) -> Result<(), Self::Error>;
    fn set_a(&self, active: bool) -> Result<(), Self::Error>;
    fn set_b(&self, active: bool) -> Result<(), Self::Error>;
    fn write(&self, data: &[u8]) -> Result<(), Self::Error>;
    fn transfer_in_place<'w>(&self, data: &'w mut [u8]) -> Result<&'w [u8], Self::Error>;
}

/// `Parts` is a container for the various parts of a SPIDriver that can be
/// used separately via distinct HAL traits.
///
/// The HAL objects inside a particular `Parts` all share a single underlying
/// communications channel, so it is not possible to access them concurrently
/// on multiple threads. Instead, coordinate all interactions with a single
/// SPIDriver on a single thread.
pub struct Parts<'a, SD: 'a>
where
    SD: Comms,
{
    /// `spi` is an implementation of the blocking SPI `Write` and `Transfer`
    /// traits with an 8-bit word size.
    pub spi: SPI<'a, SD>,

    /// `cs` is an implementation of the digital I/O `OutputPin` trait that
    /// controls the SPIDriver's Chip Select pin.
    ///
    /// Setting this pin to low is implemented as "select" on the SPIDriver and
    /// setting it to high is implemented as "unselect", for consistency with
    /// the way driver crates tend to expect a CS pin to behave.
    pub cs: CS<'a, SD>,

    /// `pin_a` is an implementation of the digital I/O `OutputPin` trait that
    /// controls the SPIDriver's auxillary output pin "A".
    pub pin_a: PinA<'a, SD>,

    /// `pin_a` is an implementation of the digital I/O `OutputPin` trait that
    /// controls the SPIDriver's auxillary output pin "B".
    pub pin_b: PinB<'a, SD>,
}

impl<'a, SD: 'a> Parts<'a, SD>
where
    SD: Comms,
{
    pub fn new(sd: &'a SD) -> Self {
        Self {
            spi: SPI::new(&sd),
            cs: CS::new(&sd),
            pin_a: PinA::new(&sd),
            pin_b: PinB::new(&sd),
        }
    }
}

/// `SPI` implements some of the SPI-related traits from `embedded-hal` in terms
/// of an SPIDriver device.
pub struct SPI<'a, SD: Comms>(&'a SD);

impl<'a, SD: 'a> SPI<'a, SD>
where
    SD: Comms,
{
    fn new(sd: &'a SD) -> Self {
        Self(sd)
    }
}

impl<'a, SD: 'a, E> spi::ErrorType for SPI<'a, SD>
where
    // SD: Comms<Error = spidriver::Error<TXErr, RXErr>>,
    SD: Comms<Error = E>,
    E: spi::Error,
{
    type Error = E;
}

// impl<'a, SD: 'a, E: spi::Error> spi::SpiDevice for SPI<'a, SD>
impl<'a, SD: 'a, E> spi::SpiDevice for SPI<'a, SD>
where
    // SD: Comms<Error = spidriver::Error<TXErr, RXErr>>,
    SD: Comms<Error = E>,
    E: spi::Error, // SpiError: From<E>, // SD: Comms<Error = E>,
{
    // type Error = E;

    /// Implements blocking SPI `Transfer` by passing the given data to the
    /// SPIDriver in chunks of up to 64 bytes each.
    ///
    /// Because of the chunking behavior, larger messages may have inconsistent
    /// timing at the chunk boundaries, which may affect devices with particularly
    /// sensitive clock timing constraints.
    fn transaction(
        &mut self,
        operations: &mut [spi::Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        for op in operations {
            match op {
                Operation::TransferInPlace(words) => {
                    // self.0.transfer_in_place(words).map_err(|_e| SpiError).map(drop)?
                    self.0.transfer_in_place(words).map(drop)?
                }
                // _ => return Err(spidriver::Error::<_, _>::Request),
                _ => {
                    // do nothing for now
                    panic!("Not supposed to happen")
                }
            }
        }
        Ok(())
    }
    // <'w>(&mut self, data: &'w mut [u8]) -> Result<&'w [u8], E> {
    //     self.0.transfer(data)
    // }
}

impl<'a, SD: 'a, E> spi::SpiBus for SPI<'a, SD>
where
    SD: Comms<Error = E>,
    E: spi::Error,
{
    fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        // Send dummy bytes while reading
        words.fill(0);
        self.0.transfer_in_place(words).map(|_| ())
    }

    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.0.write(words)
    }

    fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        // SPI requires equal-length buffers
        let len = read.len().min(write.len());

        // Copy write data into read buffer so we can perform in-place transfer
        read[..len].copy_from_slice(&write[..len]);

        self.0.transfer_in_place(&mut read[..len]).map(|_| ())
    }

    fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.0.transfer_in_place(words).map(|_| ())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        // Backend appears synchronous, so nothing to flush
        Ok(())
    }
}

// impl<'a, SD: 'a, E> spi::Write<u8> for SPI<'a, SD>
// where
//     SD: Comms<Error = E>,
// {
//     type Error = E;

//     /// Implements blocking SPI `Write` by passing the given data to the
//     /// SPIDriver in chunks of up to 64 bytes each.
//     ///
//     /// Because of the chunking behavior, larger messages may have inconsistent
//     /// timing at the chunk boundaries, which may affect devices with particularly
//     /// sensitive clock timing constraints.
//     fn write(&mut self, data: &[u8]) -> Result<(), E> {
//         self.0.write(data)
//     }
// }

/// `CS` implements some of the digital IO traits from `embedded-hal` in
/// terms of an SPIDriver device's Chip Select pin.
pub struct CS<'a, SD: Comms>(&'a SD);

impl<'a, SD: 'a> CS<'a, SD>
where
    SD: Comms,
{
    fn new(sd: &'a SD) -> Self {
        Self(sd)
    }
}

impl<'a, SD: 'a, E> digital::ErrorType for CS<'a, SD>
// where
// <SD as Comms>::Error: digital::Error,
where
    SD: Comms<Error = E>,
    E: digital::Error,
{
    // type Error = SD::Error;
    type Error = E;
}

impl<'a, SD: 'a, E> digital::OutputPin for CS<'a, SD>
where
    SD: Comms<Error = E>,
    E: digital::Error,
    // SD: Comms<Error = Self::Error>,
{
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set_cs(false)
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set_cs(true)
    }
}

// #[derive(Debug)]
// pub struct PinError;
// impl digital::Error for PinError {
//     fn kind(&self) -> digital::ErrorKind {
//         digital::ErrorKind::Other
//     }
// }

// impl<TXErr, RXErr> From<Error<TXErr, RXErr>> for SpiError {
//     fn from(value: Error<TXErr, RXErr>) -> Self {
//         SpiError
//     }
// }

/// `PinA` implements some of the digital IO traits from `embedded-hal` in
/// terms of an SPIDriver device's auxillary output pin A.
/// w#[derive(Debug)]
pub struct PinA<'a, SD: Comms>(&'a SD);

impl<'a, SD: 'a> PinA<'a, SD>
where
    SD: Comms,
{
    fn new(sd: &'a SD) -> Self {
        Self(sd)
    }
}

impl<'a, SD: 'a, E> digital::ErrorType for PinA<'a, SD>
where
    SD: Comms<Error = E>,
    E: digital::Error,
    // where
    // <SD as Comms>::Error: digital::Error,
{
    // type Error = SD::Error;
    type Error = E;
}

impl<'a, SD: 'a, E> digital::OutputPin for PinA<'a, SD>
where
    SD: Comms<Error = E>,
    E: digital::Error,
    // SD: Comms<Error = SD::Error>,
{
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set_a(false)
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set_a(true)
    }
}

/// `PinB` implements some of the digital IO traits from `embedded-hal` in
/// terms of an SPIDriver device's auxillary output pin B.
#[derive(Debug)]
pub struct PinB<'a, SD: Comms>(&'a SD);

impl<'a, SD: 'a> PinB<'a, SD>
where
    SD: Comms,
{
    fn new(sd: &'a SD) -> Self {
        Self(sd)
    }
}

impl<'a, SD: 'a, E> digital::ErrorType for PinB<'a, SD>
where
    // <SD as Comms>::Error: digital::Error,
    SD: Comms<Error = E>,
    E: digital::Error,
{
    type Error = E;
    // type Error = SD::Error;
}

impl<'a, SD: 'a, E> digital::OutputPin for PinB<'a, SD>
where
    SD: Comms<Error = E>,
    E: digital::Error,
{
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.0.set_b(false)
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.0.set_b(true)
    }
}
