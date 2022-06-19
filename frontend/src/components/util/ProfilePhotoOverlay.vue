<template>
    <v-card color="secondary" min-width="600">
        <v-app-bar elevation="0" color="secondary">
            <v-toolbar-title>
                Upload a Profile Picture
            </v-toolbar-title>

            <v-spacer> </v-spacer>

            <v-btn icon @click="close">
                <v-icon color="accent">
                    mdi-close
                </v-icon>
            </v-btn>
        </v-app-bar>
        <v-avatar 
            class="mx-5 my-5 d-flex justify-center mb-6"
            height="300"
            width="300"
        >
            <v-img 
                :src="currentPic"
            >
            </v-img>
        </v-avatar>

        <v-divider></v-divider>
        <v-file-input
            accept="image/*"
            label="File input"
            @change="updatePreview"
            v-model="currentImageFile"
        ></v-file-input>
        <v-card-actions>
            <v-spacer></v-spacer>
            <v-btn
                icon
                @click="uploadImage"
            >
                <v-icon class="accent--text">
                    mdi-check
                </v-icon>
            </v-btn>
        </v-card-actions>
        <v-snackbar v-model="errorSnackbar">
            {{ errorMessage }}

            <template v-slot:action="{ attrs }">
                <v-btn
                    color="red"
                    text
                    v-bind="attrs"
                    @click="errorSnackbar = false"
                >
                    Close
                </v-btn>
            </template>
        </v-snackbar>
    </v-card>
</template>
<script>
export default {
    props: ['userID'],
    data() {
        return {
            // urls
            imageAPIUrl: '/internal/images',

            // images
            currentPic: require('../../assets/defaultUserIcon.jpg'),
            currentImageFile: "",
            newImageUrl: "",

            // error handling
            errorMessage: "",
            errorSnackbar: ""
        }
    },
    methods: {
        close() {
            this.$emit('close')
        },

        updatePreview() {
            if (this.currentImageFile) {
                this.currentPic = URL.createObjectURL(this.currentImageFile)
            } else {
                this.currentPic = require('../../assets/defaultUserIcon.jpg')
            }
            
        },

        uploadImage() {

            const formData = new FormData()
            formData.append('file', this.currentImageFile)
            this.$http.post(this.imageAPIUrl, formData, {
                headers: {
                    'Content-Type': 'multipart/form-data',
                },
                withCredentials: true,
            }).then(response => {
                // received a photo id back
                if (response.data) {
                    const id = response.data
                    this.currentPic = `${this.imageAPIUrl}/${id}`

                    // TODO: send request to update the user photo id
                    this.changeUserProfilePic(id)
                    
                } else{
                    this.errorMessage = 'Error uploading photo'
                    this.errorSnackbar = true
                }
            }).catch(err => {
                this.errorMessage = 'Error uploading photo'
                this.errorSnackbar = true
            })
        },

        changeUserProfilePic(photoID) {
            const address = `/internal/users/${this.userID}/avatar`
            const url = `"http://${window.location.hostname}${window.location.port ? ':'+window.location.port: ''}/internal/images/${photoID}"`
            console.log(url)
            this.$http.put(address, url, {
                withCredentials: true,
                headers: {
                    'Content-Type': 'application/json'
                }
            }).then(response => {
                this.$emit('changedProfilePic')
            }).catch(err => {
                console.log(err)
            })
            // console.log(url)
        }
    }
}
</script>