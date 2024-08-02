use std::{os::windows::io::AsSocket, vec};

use rkyv::{ser::serializers::AllocSerializer, AlignedVec, Archive, Archived, CheckBytes, Deserialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{tcp::{OwnedReadHalf, OwnedWriteHalf, ReadHalf, WriteHalf}, TcpStream, UdpSocket}};

use anyhow::Result;



pub struct SequencedTcpStream {
    read: SequencedRead,
    write: SequencedWrite
}

impl SequencedTcpStream {
    pub fn new(stream: TcpStream) -> Self {
        let (read, write) = SequencedTcpStream::split(stream);
        Self {
            read, write
        }
    }
    pub fn split(stream: TcpStream) -> (SequencedRead, SequencedWrite) {

        let (read_half, write_half) = stream.into_split();
        (SequencedRead {
            read_half
        }, SequencedWrite {
            write_half
        })

    }
    pub fn into_split(self) -> (SequencedRead, SequencedWrite) {
        (self.read, self.write)
    }
    pub async fn send(&mut self, packet: &AlignedVec) -> Result<()> {
        self.write.send(packet).await
    }
    pub async fn recv(&mut self) -> Result<AlignedVec> {
        self.read.recv().await
    }
}


pub struct SequencedRead {
    read_half: OwnedReadHalf
}

impl SequencedRead {
    pub async fn recv(&mut self) -> Result<AlignedVec> {
        let packet_size: usize = self.read_half.read_u32_le().await?.try_into()?;


        // let buffer: &mut [u8] = &mut vec![0u8; packet_size.try_into()?];
        // println!("BOOFAR: {:?}", buffer);

        
        

        let mut align = AlignedVec::with_capacity(packet_size);
        for _ in 0..align.capacity() {
            align.push(0);
        }
 

        // println!("MALIGN: {:?}", align);
        self.read_half.read_exact(align.as_mut_slice()).await?;
        // println!("MALIGN: {:?}", align);

        // println!("FULL PACKET: {:?}", buffer);

        Ok(align)
        // 
        // align.extend_from_slice(self.stream.read_exact(buf))
    }
}

pub struct SequencedWrite {
    write_half: OwnedWriteHalf
}

impl SequencedWrite {
    pub async fn send_raw(&mut self, bytes: &[u8]) -> Result<()> {
        self.write_half.write_all(bytes).await?;
        Ok(())
    }
    pub async fn send(&mut self, packet: &AlignedVec) -> Result<()> {
        // println!("Packet length: {:?} {:?}", packet.len(), packet.as_ref());
        self.write_half.write_u32_le(packet.len().try_into()?).await?;
        self.write_half.write_all(packet).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {


    use std::net::SocketAddr;

    use anyhow::Result;
    use rkyv::AlignedVec;
    use tokio::net::{TcpListener, TcpStream};

    use crate::protocol::stream::SequencedTcpStream;


    #[tokio::test]
    pub async fn test_delimited() -> Result<()> {

        let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 0))).await?;

        
        let allocated_port = listener.local_addr()?.port();

  
        tokio::spawn({

            async move {

                let mut stream = SequencedTcpStream::new(TcpStream::connect(SocketAddr::from(([127, 0, 0, 1], allocated_port))).await.unwrap());
                
                let mut vec = AlignedVec::new();
                vec.push(32);

                let mut vec2 = AlignedVec::new();
                vec2.push(45);
                
                stream.send(&vec).await.unwrap();
                stream.send(&vec2).await.unwrap();

                // println!("HI {:?}", stream.recv().await.unwrap());

            }
        });

        let mut conn = SequencedTcpStream::new(listener.accept().await?.0);


        assert_eq!(*conn.recv().await?.first().unwrap(), 32);
        assert_eq!(*conn.recv().await?.first().unwrap(), 45);



        Ok(())
    }

}